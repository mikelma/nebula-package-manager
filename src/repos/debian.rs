// use regex::Regex;
use globset::{Glob, GlobSetBuilder};
use serde_derive::Deserialize;
use version_compare::{CompOp, Version};

use std::fs;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use crate::{
    pkg, utils, Dependency, DependsList, NebulaError, Package, RepoType, Repository, CONFIG,
};

// ------------------------------------------------------------------ //
//                          Configuration
// ------------------------------------------------------------------ //
#[derive(Deserialize, Clone, Debug)]
pub enum Component {
    #[serde(rename = "main")]
    Main,
    #[serde(rename = "contrib")]
    Contrib,
    #[serde(rename = "non-free")]
    NonFree,
}

impl Component {
    pub fn to_str(&self) -> &str {
        match self {
            Component::Main => "main",
            Component::Contrib => "contrib",
            Component::NonFree => "non-free",
        }
    }
}

/// Struct containing all configuration related to debian packages
#[derive(Deserialize, Clone, Debug)]
pub struct DebConfig {
    repository: String,
    components: Vec<Component>,
}

impl DebConfig {
    pub fn repository(&self) -> &str {
        self.repository.as_str()
    }

    pub fn components(&self) -> &Vec<Component> {
        &self.components
    }
}

// ------------------------------------------------------------------ //
//                          Functionalities
// ------------------------------------------------------------------ //

const DEB_REPO_DIR: &'static str = "debian";

lazy_static! {
    /// Debian repository index (Package indexes for all components are stored concatenated).
    /// The index is loaded in the first call to the variable, this avoids loading the large sized
    /// indexes repeatedly into memory.
    pub static ref DEB_REPO_INDEX: Vec<String> = {
        // get needed information about the repo to load the debian repo index
        let conf = match CONFIG.repos().debian() {
            Some(c) => c,
            None => panic!("Debian repo index has been called when \
                no debian configuration was initialized"),
        };
        let deb_repo_dir = CONFIG.repos_dir().join(DEB_REPO_DIR);

        // concatenate all package files (one for each component: main, contrib...)
        let mut files_cat: Option<fs::File> = None;
        for component in conf.components().iter() {
            let file = fs::File::open(format!(
                "{}/Packages-{}",
                deb_repo_dir.display(),
                component.to_str()
            ))
            .expect("Packages file for component not found");
            if let Some(f) = &mut files_cat {
                f.chain(file);
            } else {
                files_cat = Some(file);
            }
        }
        let reader = match files_cat {
            Some(r) => BufReader::new(r),
            None => panic!("No package files for debian repo"),
        };
        // let lines: Result<Vec<String>, std::io::Error> = reader.lines().collect();
        match reader.lines().collect() {
            Ok(s) => s,
            Err(e) => panic!("Fatal: {}", e),
        }
    };
}

pub struct Debian<'d> {
    conf: &'d DebConfig,
    // here debian configuration independent variables are defined
    repo_dir: PathBuf,
}

impl<'d> Debian<'d> {
    pub fn new() -> Result<Debian<'d>, NebulaError> {
        let conf = match &CONFIG.repos().debian() {
            Some(c) => c,
            None => return Err(NebulaError::RepoConfigNotFound),
        };

        let repo_dir = CONFIG.repos_dir().join(DEB_REPO_DIR);
        Ok(Debian { conf, repo_dir })
    }
    pub fn extract_deb(deb_path: &Path) -> Result<(), NebulaError> {
        // create a directory (with the same name of the deb to extract the deb into)
        // if it exists delete the old directory first
        let parent_dir = deb_path.parent().expect("Cannot extract root");
        let out_dir = parent_dir.join(deb_path.file_stem().unwrap());
        if out_dir.exists() {
            if let Err(e) = fs::remove_dir_all(&out_dir) {
                return Err(NebulaError::Fs(format!(
                    "cannot clean {}: {}",
                    out_dir.display(),
                    e
                )));
            }
        }
        if let Err(e) = fs::create_dir(&out_dir) {
            return Err(NebulaError::Fs(format!(
                "cannot create {}: {}",
                out_dir.display(),
                e
            )));
        }
        // extract main deb archive
        utils::cli::run_cmd(
            "/usr/bin/ar",
            &[
                "x",
                deb_path.to_str().unwrap(),
                "--output",
                out_dir.to_str().unwrap(),
            ],
        )?;

        // extract control.tar.xz
        utils::cli::run_cmd(
            "/usr/bin/tar",
            &[
                "-xf",
                &format!("{}/control.tar.xz", out_dir.display()),
                "-C",
                out_dir.to_str().unwrap(),
            ],
        )?;

        // create a new directory to extract the data tarball into
        if let Err(e) = fs::create_dir(&format!("{}/data", out_dir.display())) {
            return Err(NebulaError::Fs(e.to_string()));
        }

        // get the data tarball name, could be data.tar.{gz, xz, bz2}
        let extension = ["gz", "xz", "bz2"]
            .iter()
            .find(|e| out_dir.join(format!("data.tar.{}", e)).exists())
            .expect("Cannot find data tarball inside extracted deb");

        utils::cli::run_cmd(
            "/usr/bin/tar",
            &[
                "-xf",
                &format!("{}/data.tar.{}", out_dir.display(), extension),
                "-C",
                &format!("{}/data", out_dir.display()),
            ],
        )
    }

    /// Return's the hash of the Packages.xz archive. The hash is parsed from the InRelease file
    pub fn package_file_hash(
        releasepath: &Path,
        component: &str,
        arch: &str,
    ) -> Result<String, io::Error> {
        let packages_file = format!("{}/binary-{}/Packages.xz", component, arch);
        let file = fs::File::open(releasepath)?;
        let reader = BufReader::new(file);

        let mut sha256 = false;
        for line in reader.lines() {
            let line = line?;
            // find the SHA256 hashes, skip md5
            if line.contains("SHA256:") {
                sha256 = true;
            }

            // find the line where the hash of the package appears
            if sha256 && line.contains(&packages_file) {
                return match line.split(' ').nth(1) {
                    Some(h) => Ok(h.to_string()),
                    None => Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Not able to parse the hash",
                    )),
                };
            }
        }
        Err(io::Error::new(io::ErrorKind::Other, "hash not found"))
    }

    fn parse_dependecies_str(deps_str: &str) -> Result<Option<DependsList>, NebulaError> {
        let deps_split: Vec<&str> = deps_str.split(", ").collect();
        let mut dependencies_list = DependsList::new();
        for dep_str in deps_split {
            let mut dependency_options = vec![];
            let splitted: Vec<&str> = dep_str.split(" | ").collect();
            for pkg_str in splitted {
                let mut pkg_split = pkg_str.split_whitespace();
                // get the name of the dependency
                let dep_name = match pkg_split.next() {
                    Some(name) => name,
                    None => return Err(NebulaError::DependencyParseError),
                };
                // if exists, get packages version
                match pkg_split.next() {
                    Some(cmp_part) => {
                        let cmp_part = &cmp_part[1..].replace("<<", "<").replace(">>", ">");
                        let comp_op = match CompOp::from_sign(cmp_part) {
                            Ok(op) => op,
                            Err(_) => return Err(NebulaError::DependencyParseError),
                        };
                        // remove the opening parenthesis from string
                        match pkg_split.next() {
                            Some(ver) => dependency_options.push(Dependency::from(
                                dep_name,
                                Some((comp_op, &ver[..ver.len() - 1])),
                            )?),
                            None => return Err(NebulaError::DependencyParseError),
                        }
                    }
                    None => dependency_options.push(Dependency::from(dep_name, None)?),
                }
            }
            // add dependency options to dependency list
            if dependency_options.len() == 1 {
                dependencies_list.push(dependency_options.pop().unwrap());
            } else {
                dependencies_list.push_opts(dependency_options);
            }
        }
        if !dependencies_list.is_empty() {
            Ok(Some(dependencies_list))
        } else {
            Ok(None)
        }
    }
}

impl<'d> Repository for Debian<'d> {
    fn initialize(&self) -> Result<(), NebulaError> {
        if !self.repo_dir.is_dir() {
            fs::create_dir(&self.repo_dir).unwrap(); // create home/repo/debian
        }
        Ok(())
    }

    fn repo_type(&self) -> RepoType {
        RepoType::Debian
    }

    fn update(&self) -> Result<(), NebulaError> {
        // remove old files from debian/repo
        for entry in self.repo_dir.read_dir().expect("read_dir call failed") {
            if let Ok(entry) = entry {
                if let Err(err) = fs::remove_file(entry.path()) {
                    return Err(NebulaError::Fs(format!(
                        "Cannot clean deb repo file: {}",
                        err
                    )));
                }
            }
        }

        info!("Downloading relase file...");
        utils::download(
            format!("{}/InRelease", self.conf.repository),
            &self.repo_dir.join("InRelease"),
        );

        for component in &self.conf.components {
            info!("updating debian component {}...", component.to_str());
            // parse InRelease to get the sha256 hash of Packages.xz
            let expected_hash = Self::package_file_hash(
                &self.repo_dir.join("InRelease"),
                &component.to_str(),
                CONFIG.arch().to_str(),
            )
            .unwrap();

            // download package list for the component
            let pkgs_filename = self
                .repo_dir
                .join(format!("Packages-{}.xz", component.to_str()));
            utils::download(
                format!(
                    "{}/{}/binary-{}/Packages.xz",
                    self.conf.repository,
                    component.to_str(),
                    CONFIG.arch().to_str()
                ),
                &pkgs_filename,
            );

            // compare expected and computed hash of the downloaded file
            let real_hash = utils::fs::file2hash(Path::new(&pkgs_filename)).unwrap();
            if !real_hash.eq(&expected_hash) {
                error!("Expected and real hash of {}", pkgs_filename.display());
                return Err(NebulaError::IncorrectHash);
            }

            // extract Packages.xz file in place
            debug!("extracting {} with unxz", pkgs_filename.display());
            utils::cli::run_cmd(
                "/usr/bin/unxz",
                &["--force", pkgs_filename.to_str().unwrap()],
            )?;
        }
        Ok(())
    }

    fn search(
        &self,
        queries: &[(&str, Option<(CompOp, Version)>)],
    ) -> Result<Vec<Vec<Package>>, NebulaError> {
        let mut builder = GlobSetBuilder::new();
        for item in queries {
            builder.add(match Glob::new(format!("Package: {}", item.0).as_str()) {
                Ok(g) => g,
                Err(e) => return Err(NebulaError::GlobError(e.to_string())),
            });
        }
        let glob_set = match builder.build() {
            Ok(g) => g,
            Err(e) => return Err(NebulaError::GlobError(e.to_string())),
        };

        /*
        // concatenate all package files (one for each component: main, contrib...)
        let mut files_cat: Option<fs::File> = None;
        for component in &self.conf.components {
            let file = fs::File::open(format!(
                "{}/Packages-{}",
                self.repo_dir.display(),
                component.to_str()
            ))
            .expect("Packages file for component not found");
            if let Some(f) = &mut files_cat {
                f.chain(file);
            } else {
                files_cat = Some(file);
            }
        }
        let reader = match files_cat {
            Some(r) => BufReader::new(r),
            None => panic!("No package files for debian repo"),
        };
        // let lines: Result<Vec<String>, std::io::Error> = reader.lines().collect();
        let lines: Vec<String> = match reader.lines().collect() {
            Ok(s) => s,
            Err(e) => panic!("Fatal: {}", e),
        };
        */
        let mut pkgs_list = vec![vec![]; queries.len()];
        let mut line_index = 0;
        while line_index < DEB_REPO_INDEX.len() {
            let line = DEB_REPO_INDEX[line_index].trim_end();
            if glob_set.is_match(&line) {
                let mut match_indx = glob_set.matches(&line);
                // set package info variables
                let name = line[9..].to_string();
                // package's temporary values
                let mut version = None; // must be some at the end of the scope
                let mut source = None; // must be some at the end of the scope
                let mut depends = None;
                line_index += 1;
                while line_index < DEB_REPO_INDEX.len() && !match_indx.is_empty() {
                    let line = DEB_REPO_INDEX[line_index].trim_end();
                    if line.is_empty() {
                        break; // reached end of package info
                    }
                    if line.contains("Version: ") {
                        let capt_ver = match Version::from(&line[9..]) {
                            Some(v) => {
                                version = Some(line[9..].to_string());
                                v
                            }
                            None => return Err(NebulaError::NotSupportedVersion),
                        };
                        let mut i = 0;
                        while i < match_indx.len() {
                            if let Some((comp_op, comp_ver)) = &queries[match_indx[i]].1 {
                                if !capt_ver.compare_to(&comp_ver, &comp_op) {
                                    // if version req. is not satisfied
                                    match_indx.remove(i); // remove match from match indexes list
                                }
                            }
                            i += 1;
                        }
                    } else if line.contains("Filename: ") {
                        source = Some(pkg::PkgSource::from(
                            RepoType::Debian,
                            Some(format!("{}/{}", self.conf.repository, &line[8..]).as_str()),
                        ));
                    } else if line.starts_with("Depends: ") {
                        depends = Self::parse_dependecies_str(&line[9..])?;
                    }
                    line_index += 1;
                }
                // all info about the matched package has been read and parsed
                // check if the package still staisfies a a query
                if !match_indx.is_empty() {
                    if let Some(v) = version {
                        let source = match source {
                            Some(s) => s,
                            None => pkg::PkgSource::from(RepoType::Debian, None),
                        };
                        for mat_i in match_indx {
                            pkgs_list[mat_i].push(Package::new(
                                &name,
                                &v,
                                source.clone(),
                                depends.clone(),
                            )?);
                        }
                    } else {
                        return Err(NebulaError::VersionParsingError);
                    }
                }
            }
            line_index += 1;
        }
        Ok(pkgs_list)
    }
}

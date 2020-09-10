use regex::Regex;
use serde_derive::Deserialize;
use version_compare::{CompOp, Version};

use std::fs;
use std::io::{self, BufRead, BufReader};
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

        let repo_dir = CONFIG.repos_dir().join("debian");
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
        utils::run_cmd(
            "/usr/bin/ar",
            &[
                "x",
                deb_path.to_str().unwrap(),
                "--output",
                out_dir.to_str().unwrap(),
            ],
        )?;

        // extract control.tar.xz
        utils::run_cmd(
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

        utils::run_cmd(
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
                        // remove the opening parenthesis from string
                        match pkg_split.next() {
                            Some(ver) => dependency_options.push(Dependency::from(
                                dep_name,
                                Some(match CompOp::from_sign(&cmp_part[1..]) {
                                    Ok(op) => op,
                                    Err(_) => return Err(NebulaError::DependencyParseError),
                                }),
                                Some(&ver[..ver.len() - 1]),
                            )?),
                            None => return Err(NebulaError::DependencyParseError),
                        }
                    }
                    None => dependency_options.push(Dependency::from(dep_name, None, None)?),
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
            let real_hash = utils::file2hash(Path::new(&pkgs_filename)).unwrap();
            if !real_hash.eq(&expected_hash) {
                error!("Expected and real hash of {}", pkgs_filename.display());
                return Err(NebulaError::IncorrectHash);
            }

            // extract Packages.xz file in place
            debug!("extracting {} with unxz", pkgs_filename.display());
            utils::run_cmd(
                "/usr/bin/unxz",
                &["--force", pkgs_filename.to_str().unwrap()],
            )?;
        }
        Ok(())
    }

    fn search(
        &self,
        name: &str,
        cmp_op: &Option<CompOp>,
        version: &Option<Version>,
    ) -> Result<Option<Vec<Package>>, NebulaError> {
        fn read_line(buff: &mut dyn BufRead, line: &mut String) -> Result<usize, NebulaError> {
            line.clear();
            match buff.read_line(line) {
                Ok(n) => Ok(n),
                Err(e) => Err(NebulaError::Io(e)),
            }
        }

        // regex
        let re = Regex::new(format!(r"^Package: ({}.*)", name).as_str()).unwrap();
        let re_version = Regex::new(r"^Version: (.+)").unwrap();
        let re_src = Regex::new(r"^Filename: (.+)").unwrap();
        let re_depends = Regex::new(r"^Depends: (.+)").unwrap();

        let mut pkgs_list = vec![];
        for component in &self.conf.components {
            let mut buff = BufReader::new(
                fs::File::open(format!(
                    "{}/Packages-{}",
                    self.repo_dir.display(),
                    component.to_str()
                ))
                .expect("Package file for component not found"),
            );

            let mut line = String::new();
            while read_line(&mut buff, &mut line)? > 0 {
                // if a line matches the package name
                if re.is_match(&line) {
                    let mut continue_read = true;

                    let cap = re.captures_iter(&line).next().expect("Cannot capture name");
                    let pkg_name = cap.get(1).expect("Cannot gather name").as_str().to_string();
                    // temporary values for Package
                    let mut pkg_version = None;
                    let mut pkg_src = None;
                    let mut pkg_deps = None;
                    while read_line(&mut buff, &mut line)? > 0 && continue_read {
                        // end of info is reached
                        if line.trim_end().is_empty() {
                            continue_read = false;
                            continue;

                        // parse package's info
                        } else {
                            // get package version
                            if re_version.is_match(&line) {
                                let cap = re_version
                                    .captures_iter(&line)
                                    .next()
                                    .expect("Cannot capture version");
                                let captured_ver = cap
                                    .get(1)
                                    .expect("Cannot gather version")
                                    .as_str()
                                    .to_string();
                                // if a package version is specified
                                if let Some(target_version) = &version {
                                    let capt_v = match Version::from(&captured_ver) {
                                        Some(v) => v,
                                        None => return Err(NebulaError::NotSupportedVersion),
                                    };
                                    // check if the matched package has the target version number
                                    if let Some(cmp) = &cmp_op {
                                        // if a comp. op. is specified compare versions
                                        if capt_v.compare(&target_version) == *cmp {
                                            pkg_version = Some(captured_ver)
                                        }
                                    } else {
                                        // no comp. op. is supplied, so check if both versions are
                                        // equal
                                        if capt_v.compare(&target_version) == CompOp::Eq {
                                            pkg_version = Some(captured_ver)
                                        }
                                    }
                                } else {
                                    // the package does not have to match a version, all versions
                                    // are valid
                                    pkg_version = Some(captured_ver);
                                }
                            }
                            // get package source
                            if re_src.is_match(&line) {
                                let cap = re_src
                                    .captures_iter(&line)
                                    .next()
                                    .expect("Cannot capture source (Filename)");
                                let pkg_url = format!(
                                    "{}/{}",
                                    self.conf.repository,
                                    cap.get(1)
                                        .expect("Cannot gather source (Filename)")
                                        .as_str()
                                );
                                pkg_src = Some(pkg::PkgSource::from(RepoType::Debian, &pkg_url));
                            }
                            // get dependencies
                            if re_depends.is_match(&line) {
                                let cap = re_depends
                                    .captures_iter(&line)
                                    .next()
                                    .expect("Cannot capture dependencies");
                                let deps_str = cap
                                    .get(1)
                                    .expect("Cannot gather dependencies list")
                                    .as_str();
                                pkg_deps = Self::parse_dependecies_str(&deps_str)?;
                            }
                        }
                    }
                    if let Some(ver) = pkg_version {
                        if let Some(src) = pkg_src {
                            pkgs_list.push(Package::new(&pkg_name, &ver, src, pkg_deps)?);
                        } else {
                            return Err(NebulaError::SourceParsingError);
                        }
                    }
                }
            }
        }
        if pkgs_list.is_empty() {
            Ok(None)
        } else {
            Ok(Some(pkgs_list))
        }
    }
}

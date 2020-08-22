use serde_derive::Deserialize;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

use crate::errors::NebulaError;
use crate::Repository;
use crate::CONFIG;
use crate::{download, file2hash};

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
    pub repository: String,
    pub components: Vec<Component>,
}

// ------------------------------------------------------------------ //
//                          Functionalities
// ------------------------------------------------------------------ //

pub struct Debian {}

impl Repository for Debian {
    fn initialize(&self) -> Result<(), NebulaError> {
        let homedir = match CONFIG.repos.debian {
            Some(_) => CONFIG.nebulahome.clone(),
            None => return Err(NebulaError::RepoConfigNotFound),
        };

        let deb_repo_path = homedir.join("repo/debian");
        if !deb_repo_path.is_dir() {
            fs::create_dir(deb_repo_path).unwrap(); // create home/repo/debian
        }
        Ok(())
    }

    fn update(&self) -> Result<(), NebulaError> {
        println!("[*] updating debian repositories");
        // env::set_current_dir(CONFIG.nebulahome.join("repo/debian")).unwrap();
        // extract debian specific config for latter use
        let deb_config = match &CONFIG.repos.debian {
            Some(c) => c,
            None => return Err(NebulaError::RepoConfigNotFound),
        };
        let repo_dir = CONFIG.nebulahome.join("repo/debian");

        // remove old files from debian/repo
        for entry in repo_dir.read_dir().expect("read_dir call failed") {
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
        download(
            format!("{}/InRelease", deb_config.repository),
            &repo_dir.join("InRelease"),
        );

        for component in &deb_config.components {
            info!("updating debian component {}...", component.to_str());
            // parse InRelease to get the sha256 hash of Packages.xz
            let expected_hash = Self::package_file_hash(
                &repo_dir.join("InRelease"),
                &component.to_str(),
                CONFIG.arch.to_str(),
            )
            .unwrap();

            // download package list for the component
            let pkgs_filename = repo_dir.join(format!("Packages-{}.xz", component.to_str()));
            download(
                format!(
                    "{}/{}/binary-{}/Packages.xz",
                    deb_config.repository,
                    component.to_str(),
                    CONFIG.arch.to_str()
                ),
                &pkgs_filename,
            );

            // compare expected and computed hash of the downloaded file
            let real_hash = file2hash(Path::new(&pkgs_filename)).unwrap();
            if !real_hash.eq(&expected_hash) {
                error!("Expected and real hash of {}", pkgs_filename.display());
                return Err(NebulaError::IncorrectHash);
            }

            // extract Packages.xz file in place
            debug!("extracting {} with unxz", pkgs_filename.display());
            crate::run_cmd(
                "/usr/bin/unxz",
                &["--force", pkgs_filename.to_str().unwrap()],
            )?;
        }
        Ok(())
    }
}

impl Debian {
    pub fn new() -> Result<Debian, NebulaError> {
        if CONFIG.repos.debian.is_some() {
            Ok(Debian {})
        } else {
            Err(NebulaError::RepoConfigNotFound)
        }
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
        crate::run_cmd(
            "/usr/bin/ar",
            &[
                "x",
                deb_path.to_str().unwrap(),
                "--output",
                out_dir.to_str().unwrap(),
            ],
        )?;

        // extract control.tar.xz
        crate::run_cmd(
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

        crate::run_cmd(
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
}

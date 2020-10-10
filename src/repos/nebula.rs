use serde_derive::Deserialize;
use version_compare::{CompOp, Version};

use std::error::Error;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;

use crate::pkgdb::PkgDB;
use crate::{errors::*, pkg, utils, Package, RepoType, Repository, CONFIG};

#[derive(Deserialize, Clone, Debug)]
pub struct NebulaConfig {
    pub repository: String,
}

/// Name of the directory inside nbpm's home that contains all
/// info about nebula's repository
const NB_REPO_DIR: &'static str = "nebula";
/// Name of the file that contains the repository index of a nebula repository
const NB_PKG_INDEX_NAME: &'static str = "packages.toml";

lazy_static! {
    /// Nebula's repository index
    pub static ref NB_REPO_INDEX: PkgDB = {
        let filepath = Path::new(CONFIG.repos_dir()).join(NB_REPO_DIR).join(NB_PKG_INDEX_NAME);
        match PkgDB::from(&filepath) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Error loading nebula repo index: {} {}", e, filepath.display());
                exit(1);
            },
        }
    };
}

pub struct RepoNebula<'d> {
    /// path of directory where all files relative to the nebula repository are
    repo_dir: PathBuf,
    /// reference to the nebula repo configuration (NebulaConfig) inside the main configuration
    /// (CONFIG object)
    conf: &'d NebulaConfig,
}

impl<'d> RepoNebula<'d> {
    pub fn new() -> RepoNebula<'d> {
        let conf = &CONFIG.repos().nebula();
        let repo_dir = CONFIG.repos_dir().join(NB_REPO_DIR);
        // atributte index takes None as initial value, when the value is first called via index
        // method, the atributte will take the PkgDB value (lazy loading).
        RepoNebula { conf, repo_dir }
    }

    pub fn repo_dir(&self) -> &Path {
        self.repo_dir.as_path()
    }
}

impl<'d> Repository for RepoNebula<'d> {
    fn initialize(&self) -> Result<(), Box<dyn Error>> {
        if !self.repo_dir.is_dir() {
            if let Err(e) = fs::create_dir(&self.repo_dir) {
                return Err(Box::new(e));
            }
        }
        Ok(())
    }

    fn repo_type(&self) -> RepoType {
        RepoType::Nebula
    }

    fn update(&self) -> Result<(), Box<dyn Error>> {
        // remove old files from debian/repo
        for entry in self.repo_dir.read_dir().expect("read_dir call failed") {
            if let Ok(entry) = entry {
                if let Err(e) = fs::remove_file(entry.path()) {
                    return Err(Box::new(e));
                }
            }
        }

        info!("Downloading nebula's packages file...");
        utils::download(
            format!("{}/{}", self.conf.repository, NB_PKG_INDEX_NAME),
            &self.repo_dir.join(NB_PKG_INDEX_NAME),
        )?;
        Ok(())
    }

    fn search(
        &self,
        queries: &[(&str, Option<(CompOp, Version)>)],
    ) -> Result<Vec<Vec<Package>>, Box<dyn Error>> {
        NB_REPO_INDEX.search(&queries)
    }
}

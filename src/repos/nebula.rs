use serde_derive::Deserialize;
use version_compare::{CompOp, Version};

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;

use crate::pkgdb::PkgDB;
use crate::{pkg, utils, NebulaError, Package, RepoType, Repository, CONFIG};

#[derive(Deserialize, Clone, Debug)]
pub struct NebulaConfig {
    pub repository: String,
}

const PKG_INDEX_NAME: &'static str = "packages.toml";
lazy_static! {
    /// Nebula's repository index
    pub static ref NB_REPO_INDEX: PkgDB = match PkgDB::from(Path::new(PKG_INDEX_NAME).as_ref()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Fatal error: {:?}", e);
            exit(1);
        },
    };
}

pub struct RepoNebula<'d> {
    repo_dir: PathBuf,
    conf: &'d NebulaConfig,
}

impl<'d> RepoNebula<'d> {
    pub fn new() -> Result<RepoNebula<'d>, NebulaError> {
        let conf = &CONFIG.repos().nebula();
        let repo_dir = CONFIG.repos_dir().join("nebula");
        Ok(RepoNebula { conf, repo_dir })
    }
}

impl<'d> Repository for RepoNebula<'d> {
    fn initialize(&self) -> Result<(), NebulaError> {
        if !self.repo_dir.is_dir() {
            if let Err(e) = fs::create_dir(&self.repo_dir) {
                return Err(NebulaError::Fs(e.to_string()));
            }
        }
        Ok(())
    }

    fn repo_type(&self) -> RepoType {
        RepoType::Nebula
    }

    fn update(&self) -> Result<(), NebulaError> {
        // remove old files from debian/repo
        for entry in self.repo_dir.read_dir().expect("read_dir call failed") {
            if let Ok(entry) = entry {
                if let Err(err) = fs::remove_file(entry.path()) {
                    return Err(NebulaError::Fs(format!(
                        "Cannot clean nebula repo file: {}",
                        err
                    )));
                }
            }
        }

        info!("Downloading nebula's packages file...");
        utils::download(
            format!("{}/{}", self.conf.repository, PKG_INDEX_NAME),
            &self.repo_dir.join(PKG_INDEX_NAME),
        );
        Ok(())
    }

    fn search(
        &self,
        queries: &[(&str, Option<(CompOp, Version)>)],
    ) -> Result<Vec<Vec<Package>>, NebulaError> {
        NB_REPO_INDEX.search(&queries)
    }
}

use semver::VersionReq;
use serde_derive::{Deserialize, Serialize};

use std::error::Error;
use std::fmt;

use crate::{Package, CONFIG};

pub mod debian;
pub mod nebula;
pub mod rosetta;

pub use debian::{DebConfig, Debian};
pub use nebula::{NebulaConfig, RepoNebula};

pub type Query<'a> = (&'a str, VersionReq);

pub trait Repository {
    fn initialize(&self) -> Result<(), Box<dyn Error>>;
    fn repo_type(&self) -> RepoType;
    fn update(&self) -> Result<(), Box<dyn Error>>;
    fn search(&self, queries: &[Query]) -> Result<Vec<Vec<Package>>, Box<dyn Error>>;
    /*
    fn install(packages: &[Package]) -> Result<(), NebulaError> {
        for pkg in packages {
            println!("    - install {}", pkg.name());
        }
        Ok(())
    }
    */
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone, Eq, Hash)]
pub enum RepoType {
    #[serde(rename(deserialize = "nebula"))]
    Nebula,
    #[serde(rename(deserialize = "debian"))]
    Debian,
}

impl Default for RepoType {
    fn default() -> Self {
        RepoType::Nebula
    }
}

impl fmt::Display for RepoType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepoType::Nebula => write!(f, "nebula"),
            RepoType::Debian => write!(f, "debian"),
        }
    }
}

pub fn create_repos() -> Result<Vec<Box<dyn Repository>>, Box<dyn Error>> {
    let mut repos: Vec<Box<dyn Repository>> = vec![];
    repos.push(Box::new(RepoNebula::new()));
    // debian repo
    if CONFIG.repos().debian().is_some() {
        repos.push(Box::new(Debian::new()?));
    }

    Ok(repos)
}

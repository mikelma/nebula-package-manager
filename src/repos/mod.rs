use std::fmt;
use version_compare::{CompOp, Version};

pub mod debian;
pub mod nebula;

pub use debian::{DebConfig, Debian};
pub use nebula::{NebulaConfig, RepoNebula};

use crate::{NebulaError, Package, CONFIG};

pub trait Repository {
    fn initialize(&self) -> Result<(), NebulaError>;
    fn repo_type(&self) -> RepoType;
    fn update(&self) -> Result<(), NebulaError>;
    fn search(
        &self,
        queries: &[(&str, Option<(CompOp, Version)>)],
    ) -> Result<Vec<Vec<Package>>, NebulaError>;
    /*
    fn install(packages: &[Package]) -> Result<(), NebulaError> {
        for pkg in packages {
            println!("    - install {}", pkg.name());
        }
        Ok(())
    }
    */
}

#[derive(Debug, PartialEq, Clone)]
pub enum RepoType {
    Debian,
    Nebula,
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

pub fn create_repos() -> Result<Vec<Box<dyn Repository>>, NebulaError> {
    let mut repos: Vec<Box<dyn Repository>> = vec![];
    repos.push(Box::new(RepoNebula::new()?));
    // debian repo
    if CONFIG.repos().debian().is_some() {
        repos.push(Box::new(Debian::new()?));
    }

    Ok(repos)
}

use std::fmt;

pub mod debian;
pub mod nebula;

pub use debian::{DebConfig, Debian};
pub use nebula::NebulaConfig;

use crate::{NebulaError, Package, CONFIG};

pub trait Repository {
    fn initialize(&self) -> Result<(), NebulaError>;
    fn repo_type(&self) -> RepoType;
    fn update(&self) -> Result<(), NebulaError>;
    fn search(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<Option<Vec<Package>>, NebulaError>;
}

#[derive(Debug, PartialEq, Clone)]
pub enum RepoType {
    Debian,
    Nebula,
}

impl fmt::Display for RepoType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepoType::Nebula => write!(f, "nebula"),
            RepoType::Debian => write!(f, "debian"),
        }
    }
}

pub fn create_repos() -> Result<Vec<impl Repository>, NebulaError> {
    let mut repos = vec![];
    // TODO: Nebula repository init
    // debian repo
    if CONFIG.repos().debian().is_some() {
        repos.push(Debian::new()?);
    }

    Ok(repos)
}

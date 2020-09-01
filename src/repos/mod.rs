pub mod debian;
pub mod nebula;

pub use debian::{DebConfig, Debian};
pub use nebula::NebulaConfig;

use crate::{NebulaError, Package, CONFIG};

pub trait Repository {
    fn initialize(&self) -> Result<(), NebulaError>;
    fn update(&self) -> Result<(), NebulaError>;
    fn search(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<Option<Vec<Package>>, NebulaError>;
}

#[derive(Debug, PartialEq)]
pub enum RepoType {
    Debian,
    Nebula,
}

pub fn create_repos() -> Result<Vec<impl Repository>, NebulaError> {
    let mut repos = vec![];
    // TODO: Nebula repository init
    // debian repo
    if CONFIG.repos.debian.is_some() {
        repos.push(Debian::new()?);
    }

    Ok(repos)
}

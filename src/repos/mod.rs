use serde_derive::{Deserialize, Serialize};

pub mod debian;
pub mod nebula;

pub use debian::{DebConfig, Debian};
pub use nebula::NebulaConfig;

use crate::{NebulaError, Package, CONFIG};

pub trait Repository {
    fn get_type(&self) -> RepoType;
    fn initialize(&self) -> Result<(), NebulaError>;
    fn update(&self) -> Result<(), NebulaError>;
    fn search(&self, name: &str, version: Option<&str>) -> Result<Option<Package>, NebulaError>;
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub enum RepoType {
    #[serde(rename = "debian")]
    Debian,
    #[serde(rename = "nebula")]
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

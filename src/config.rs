use serde_derive::Deserialize;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

use crate::errors::NebulaError;
use crate::repos::{DebConfig, NebulaConfig};

#[derive(Deserialize, Debug)]
pub enum Arch {
    #[serde(rename = "amd64")]
    Amd64,
}

impl Arch {
    pub fn to_str(&self) -> &str {
        match self {
            Arch::Amd64 => "amd64",
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct RepoConfigs {
    pub nebula: NebulaConfig,
    // debian repository configs
    // #[serde(rename = "Debian")]
    pub debian: Option<DebConfig>,
}

#[derive(Deserialize, Debug)]
pub struct Configuration {
    // system configuration
    pub arch: Arch,

    // nebula paths
    #[serde(rename = "fakeroot-dir")]
    pub fakerootdir: PathBuf,
    #[serde(rename = "destination-dir")]
    pub destdir: PathBuf,
    #[serde(rename = "nebula-dir")]
    pub nebulahome: PathBuf,

    // repository configurations
    #[serde(rename = "repositories")]
    pub repos: RepoConfigs,
}

impl Configuration {
    pub fn from(path: &Path) -> Result<Configuration, NebulaError> {
        // read configuration file
        match read_to_string(path) {
            // deserialize configuration file
            Ok(s) => match toml::from_str(s.as_str()) {
                Ok(c) => Ok(c),
                Err(e) => Err(NebulaError::TomlDe(e)),
            },
            Err(e) => Err(NebulaError::Io(e)),
        }
    }
}

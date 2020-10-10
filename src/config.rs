use serde_derive::{Deserialize, Serialize};

use std::error::Error;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

use crate::repos::{DebConfig, NebulaConfig};

pub mod constants {
    use super::Configuration;
    use std::path::Path;

    /// The path where the configuratio file of nebula should be
    pub static CONFIG_PATH: &str = "config.toml";

    lazy_static! {
        /// Contains all information of the nb package manager configuration file
        pub static ref CONFIG: Configuration =
            Configuration::from(Path::new(CONFIG_PATH)).unwrap();
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    nebula: NebulaConfig,
    // debian repository configs
    // #[serde(rename = "Debian")]
    debian: Option<DebConfig>,
}

impl RepoConfigs {
    pub fn nebula(&self) -> &NebulaConfig {
        &self.nebula
    }

    pub fn debian(&self) -> &Option<DebConfig> {
        &self.debian
    }
}

#[derive(Deserialize, Debug)]
pub struct Configuration {
    // system configuration
    arch: Arch,

    // nebula paths
    #[serde(rename = "fakeroot-dir")]
    fakerootdir: PathBuf,
    #[serde(rename = "destination-dir")]
    destdir: PathBuf,
    #[serde(rename = "nebula-dir")]
    nebulahome: PathBuf,

    // repository configurations
    #[serde(rename = "repositories")]
    repos: RepoConfigs,

    // -- defaults -- //
    // this fields are skipped by serde, when contructing a `Configuration` instance calling
    // `Configuration::from` the fields will be set to its default values automatically.
    #[serde(skip, default = "set_defaults_empty")]
    repos_dir: Option<PathBuf>,
    #[serde(skip, default = "set_defaults_empty")]
    pkgignore: Option<PathBuf>,
    #[serde(skip, default = "set_defaults_empty")]
    logfile: Option<PathBuf>,
}

impl Configuration {
    pub fn from(path: &Path) -> Result<Configuration, Box<dyn Error>> {
        // read configuration file
        match read_to_string(path) {
            // deserialize configuration file
            Ok(s) => match toml::from_str::<Configuration>(s.as_str()) {
                Ok(mut c) => {
                    c.default();
                    Ok(c)
                }
                Err(e) => Err(Box::new(e)),
            },
            Err(e) => Err(Box::new(e)),
        }
    }

    // --- getters --- //

    fn default(&mut self) {
        self.repos_dir = Some(self.nebulahome.join("repos"));
        self.pkgignore = Some(self.nebulahome.join("pkgignore"));
        self.logfile = Some(self.nebulahome.join("nebula.log"));
    }

    pub fn fakerootdir(&self) -> &Path {
        self.fakerootdir.as_path()
    }

    pub fn destdir(&self) -> &Path {
        self.destdir.as_path()
    }

    pub fn nebulahome(&self) -> &Path {
        self.nebulahome.as_path()
    }

    pub fn repos(&self) -> &RepoConfigs {
        &self.repos
    }

    pub fn arch(&self) -> &Arch {
        &self.arch
    }

    pub fn repos_dir(&self) -> &Path {
        match &self.repos_dir {
            Some(d) => d.as_path(),
            None => panic!(
                "Default field `repos_dir` empty in Configuration. \
                Configuration objects must be contructed with Configuration::from"
            ),
        }
    }

    pub fn pkgignore(&self) -> &Path {
        match &self.pkgignore {
            Some(d) => d.as_path(),
            None => panic!(
                "Default field `pkgignore` empty in Configuration. \
                Configuration objects must be contructed with Configuration::from"
            ),
        }
    }

    pub fn logfile(&self) -> &Path {
        match &self.logfile {
            Some(d) => d.as_path(),
            None => panic!(
                "Default field `logfile` empty in Configuration. \
                Configuration objects must be contructed with Configuration::from"
            ),
        }
    }
}

// used to set initial values of default atributtes in `Configuration`
fn set_defaults_empty() -> Option<PathBuf> {
    Option::None
}

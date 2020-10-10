#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use simplelog::*;

use std::error::Error;
use std::fs::{create_dir, create_dir_all, File};

pub mod config;
pub mod errors;
pub mod pkg;
pub mod pkgdb;
pub mod repos;
pub mod utils;

pub use config::constants::*;
pub use pkg::{Dependency, DependsList, Package};
pub use repos::{create_repos, RepoType, Repository};

/// Checks if all nebula directories are present, if not, creates the needed directories. It also
/// creates the needed files, such as the logger.
pub fn initialize(repos: &Vec<Box<dyn Repository>>) -> Result<(), Box<dyn Error>> {
    // check nebula's home and cache directory (inside home directory)
    if !CONFIG.nebulahome().is_dir() {
        create_dir_all(&CONFIG.nebulahome())?; // create home
        create_dir(&CONFIG.repos_dir())?; // create home/repo
    }

    // check fakeroot
    if !CONFIG.fakerootdir().is_dir() {
        create_dir_all(&CONFIG.fakerootdir())?;
    }

    // check destdir, if it does not exists, fatal error: panic!
    if !CONFIG.destdir().is_dir() {
        eprintln!(
            "Fatal: The destination directory for packages does not exist, 
            please create or change the destination directory: {}",
            CONFIG.destdir().display()
        );
    }

    // create the logger
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Warn, Config::default(), TerminalMode::Mixed),
        WriteLogger::new(
            LevelFilter::Debug,
            Config::default(),
            File::create(&CONFIG.logfile())?,
        ),
    ])?;

    // initi all repos
    for repo in repos {
        repo.initialize()?;
    }

    Ok(())
}

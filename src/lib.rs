#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

use curl::easy::Easy;
use fs::OpenOptions;
use sha2::{Digest, Sha256};
use simplelog::*;
use std::fs::{self, create_dir, create_dir_all, File};
use std::io::{Read, Write};
use std::os::unix;
use std::path::{Path, PathBuf};
use std::process;
use std::process::Command;
use walkdir::WalkDir;

pub mod config;
pub mod errors;
pub mod pkg;
pub mod repos;

pub use errors::NebulaError;
pub use pkg::Package;
pub use repos::{RepoType, Repository};

// pub mod nebula;
use config::Configuration;

lazy_static! {
    pub static ref CONFIG: Configuration = Configuration::from(Path::new("config.toml")).unwrap();
}

/// Checks if all nebula directories are present, if not, creates the needed directories. It also
/// creates the needed files, such as the logger.
pub fn initialize(repos: &[impl Repository]) -> Result<(), NebulaError> {
    // check nebula's home and cache directory (inside home directory)
    if !CONFIG.nebulahome.is_dir() {
        create_dir_all(&CONFIG.nebulahome).unwrap(); // create home
        create_dir(&CONFIG.nebulahome.join("repo")).unwrap(); // create home/repo
    }

    // check fakeroot
    if !CONFIG.fakerootdir.is_dir() {
        create_dir_all(&CONFIG.fakerootdir).unwrap();
    }

    // check destdir, if it does not exists, fatal error: panic!
    if !CONFIG.destdir.is_dir() {
        eprintln!(
            "Fatal: The destination directory for packages does not exist, 
            please create or change the destination directory: {}",
            CONFIG.destdir.display()
        );
    }

    // create the logger
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Warn, Config::default(), TerminalMode::Mixed),
        WriteLogger::new(
            LevelFilter::Debug,
            Config::default(),
            File::create("nebula.log").unwrap(),
        ),
    ])
    .unwrap();

    // initi all repos
    for repo in repos {
        repo.initialize()?;
    }

    Ok(())
}

pub fn download(url: String, outfile: &Path) {
    // delete the file/dir to download if it already exists
    if outfile.is_dir() && outfile.exists() {
        fs::remove_dir_all(&outfile).unwrap();
    }
    if outfile.is_file() && outfile.exists() {
        fs::remove_file(&outfile).unwrap();
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(&outfile)
        .unwrap();
    let mut handle = Easy::new();
    handle.url(&url).unwrap();
    {
        let mut transfer = handle.transfer();
        transfer
            .write_function(|new_data| {
                file.write_all(new_data).unwrap();
                Ok(new_data.len())
            })
            .unwrap();
        transfer.perform().unwrap();
    }
}

pub fn create_links(src: &Path, dest: &Path) {
    // get absolute form of paths
    let src = fs::canonicalize(src).unwrap();
    let dest = fs::canonicalize(dest).unwrap();

    let mut links = Vec::<PathBuf>::new();
    for src_entry in WalkDir::new(&src) {
        let src_entry = src_entry.unwrap();
        // remove src directory pat from entry
        let path = src_entry.path().strip_prefix(&src).unwrap();

        //println!("{}", path.display());
        let new_path = dest.join(path);

        // check if the new path is inside a created link
        if links.iter().find(|l| new_path.starts_with(l)).is_none() {
            // check if the new file already exists in the source dir
            if !new_path.exists() {
                // try to create the symlink
                match unix::fs::symlink(src_entry.path(), &new_path) {
                    Ok(()) => {
                        debug!(
                            "new link: {} -> {}",
                            src_entry.path().display(),
                            new_path.display()
                        );
                        links.push(new_path);
                    }
                    Err(e) => {
                        error!("symlink error: {}", e);
                        error!(
                            "can't link: {} -> {}",
                            src_entry.path().display(),
                            new_path.display()
                        );
                        info!("Cleaning created symlinks");
                        eprintln!("[!] An error occurred while creating the links");
                        let mut exit_ok = true;
                        // An error occurred, destoy every link created until now
                        links.iter().for_each(|l| {
                            // if the link can not be removed, notify the user and continue
                            // removing other links
                            if fs::remove_file(l).is_err() {
                                error!("Could not unlik: {}, manual removal needed!", l.display());
                                exit_ok = false;
                            }
                        });
                        if exit_ok {
                            eprintln!("[!] Exiting successfully...");
                            info!("Links cleaned successfully");
                        } else {
                            eprintln!("[!] Fatal: Could not create links successfully and some links could not be cleaned...");
                            warn!("Could not create links successfully and some links could not be cleaned");
                        }
                        process::exit(1);
                    }
                }
            }
        }
    }
}

/// Computes the Sha256 hash of the given file.
pub fn file2hash(filepath: &Path) -> Result<String, ()> {
    let mut file = fs::File::open(filepath).unwrap();
    let mut buffer = Vec::<u8>::new();
    file.read_to_end(&mut buffer).unwrap();
    Ok(format!("{:x}", Sha256::digest(&buffer)))
}

pub fn run_cmd(cmd: &str, args: &[&str]) -> Result<(), NebulaError> {
    // create the command and add arguments if necessary
    let mut command = Command::new(cmd);
    if !args.is_empty() {
        command.args(args);
    }
    // execute command as child process
    let child = match command.output() {
        Ok(c) => c,
        Err(e) => {
            return Err(NebulaError::CmdError(format!(
                "failed to start {}: {}",
                cmd, e
            )));
        }
    };
    // read status and return result
    if child.status.success() {
        Ok(())
    } else {
        let message = String::from_utf8_lossy(&child.stderr);
        Err(NebulaError::CmdError(message.to_string()))
    }
}

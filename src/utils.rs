use curl::easy::Easy;
use sha2::{Digest, Sha256};

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

use crate::{NebulaError, Package};

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
                        std::process::exit(1);
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
pub fn choose_from_table(pkgs: &[Package]) -> Result<usize, NebulaError> {
    for (id, pkg) in pkgs.iter().enumerate() {
        println!(
            "[{}] {} {} (deps.: {})",
            id,
            pkg.name(),
            pkg.version(),
            match pkg.depends() {
                Some(lst) => lst.len(),
                None => 0,
            }
        );
    }
    let id: usize;
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        let mut line = String::new();
        let _n = std::io::stdin().read_line(&mut line).unwrap();
        line = line.trim_end().to_string();
        match line.parse::<usize>() {
            Ok(n) if n > 0 && n <= pkgs.len() => {
                id = n;
                break;
            }
            Ok(_) | Err(_) => continue,
        }
    }
    Ok(id)
}

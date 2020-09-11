use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use std::fs;
use std::io::Read;
use std::os::unix;
use std::path::{Path, PathBuf};

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

// use curl::easy::Easy;
use reqwest;
use version_compare::{CompOp, Version};

use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use crate::errors::*;

pub mod cli;
pub mod fs;
pub mod resolve;
pub mod search;

/// parse information of a package given a string. The string format must be: pkg_name or
/// [pkgname][comp_op][version]. Examples: "neofetch", "glibc", "linux>=5.5.3" and "make<1.0".
pub fn parse_pkg_str_info(text: &str) -> Result<(&str, Option<(CompOp, &str)>), NebulaError> {
    // search for comparison operator on the query
    // NOTE: May use Regex in the future
    let mut name = text;
    let mut comp_ver = None;
    for operator in &["==", ">=", "<=", ">", "<"] {
        // if an operator is present extract the name, comparison operator and version
        if text.contains(operator) {
            let mut splitted = text.split(operator);
            name = splitted.next().unwrap();
            comp_ver = match splitted.next() {
                Some("") | None => {
                    return Err(NebulaError::from_msg(
                        "comparison operator found, but version is missing",
                        NbErrType::Parsing,
                    ));
                }
                Some(v) => match Version::from(v) {
                    Some(_) => Some((CompOp::from_sign(operator).unwrap(), v)),
                    None => {
                        return Err(NebulaError::from_msg(v, NbErrType::VersionFmt));
                    }
                },
            };
            break;
        }
    }
    Ok((name, comp_ver))
}

pub fn download(url: String, outfile: &Path) -> Result<(), Box<dyn Error>> {
    // delete the file/dir to download if it already exists
    if outfile.is_dir() && outfile.exists() {
        std::fs::remove_dir_all(&outfile)?;
    }
    if outfile.is_file() && outfile.exists() {
        std::fs::remove_file(&outfile)?;
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(&outfile)?;
    let body = reqwest::blocking::get(&url)?;
    file.write_all(&body.bytes()?)?;
    Ok(())
}

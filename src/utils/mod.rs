// use curl::easy::Easy;
use reqwest;
use semver::{Version, VersionReq};

use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use crate::errors::*;
use crate::repos::Query;

pub mod cli;
pub mod fs;
pub mod resolve;
pub mod search;

/// parse information of a package given a string. The string format must be: pkg_name or
/// [pkgname][comp_op][version]. Examples: "neofetch", "glibc", "linux>=5.5.3" and "make<1.0".
pub fn parse_pkg_str_info(text: &str) -> Result<Query, Box<dyn Error>> {
    // search for comparison operator on the query
    for operator in &["==", ">=", "<=", ">", "<"] {
        // if an operator is present extract the name, comparison operator and version
        if text.contains(operator) {
            // split the name and versionreq part
            let mut splitted = text.split(operator);
            let name = splitted.next().unwrap();
            /*
            comp_ver = match splitted.next() {
                Some("") | None => {
                    return Err(NebulaError::from_msg(
                        "comparison operator found, but version is missing",
                        NbErrType::Parsing,
                    ));
                }
                Some(v) => match Version::from(v) {
                    Some(ver) => Some((CompOp::from_sign(operator).unwrap(), ver)),
                    None => {
                        return Err(NebulaError::from_msg(v, NbErrType::VersionFmt));
                    }
                },
                */
            let comp_ver = match splitted.next() {
                Some(s) => VersionReq::parse(s)?,
                None => VersionReq::any(),
            };
            return Ok((name, comp_ver));
        }
    }
    Err(Box::new(NebulaError::from_msg(
        "Incorrect comparison operator",
        NbErrType::VersionFmt,
    )))
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

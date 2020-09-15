use curl::easy::Easy;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

pub mod cli;
pub mod fs;
pub mod resolve;

pub fn download(url: String, outfile: &Path) {
    // delete the file/dir to download if it already exists
    if outfile.is_dir() && outfile.exists() {
        std::fs::remove_dir_all(&outfile).unwrap();
    }
    if outfile.is_file() && outfile.exists() {
        std::fs::remove_file(&outfile).unwrap();
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

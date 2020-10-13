#[macro_use]
extern crate clap;

use clap::Arg;

use std::path::Path;

use nbpm::{exit_with_err, utils};

fn main() {
    let cli_args = app_from_crate!()
        .about("Create symlinks from source directory files to a destination directory")
        .arg(
            Arg::with_name("SRC")
                .required(true)
                .help("Directory where all source files are located"),
        )
        .arg(
            Arg::with_name("DEST")
                .required(true)
                .help("Destination directory where the links to the sources are going to be placed on"),
        )
        .get_matches();
    
    
    let src = cli_args.value_of("SRC").unwrap();
    let dest = cli_args.value_of("DEST").unwrap();
    
    println!("[*] Creating links from {} to {}", src, dest);
    match utils::fs::create_links(Path::new(src), Path::new(dest)) {
        Ok(_) => println!("[*] Links created successfully!"),
        Err(e) => exit_with_err(e),
    }
}

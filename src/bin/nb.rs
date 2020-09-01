#[macro_use]
extern crate clap;
extern crate regex;

use clap::{App, Arg, SubCommand};

use nbpm::Repository;

fn main() {
    let cli_args = app_from_crate!()
        .arg(
            Arg::with_name("update-repos")
                .short("u")
                .long("update")
                .help("Update all repostories")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("pkg-name")
                .short("s")
                .long("search")
                .value_name("PKG")
                .help("search for a package matching PKG")
                .takes_value(true),
        )
        .get_matches();

    let repos = nbpm::repos::create_repos().unwrap();
    nbpm::initialize(&repos).unwrap();

    // update repositories
    if cli_args.is_present("update-repos") {
        print!("[*] Updating repositories... ");
        for repo in &repos {
            repo.update().unwrap();
        }
        println!("done");
    }

    // search for a package
    if let Some(query) = cli_args.value_of("pkg-name") {
        for repo in repos {
            match repo.search(query, None) {
                Ok(Some(pkgs)) => pkgs.iter().for_each(|p| println!("{}\n", p)),
                Ok(None) => println!("No package matching '{}' in this repository", query),
                Err(err) => eprintln!("Error: {:?}", err),
            }
        }
    }
}

#[macro_use]
extern crate clap;
extern crate regex;

use clap::Arg;
/*
use cli_table::{
    format::{self, CellFormat, Justify},
    Cell, Row, Table,
};
*/
use version_compare::{CompOp, Version};

use std::io::{stdin, stdout, Write};
use std::process::exit;

use nbpm::{utils, Package, Repository};

fn main() {
    let cli_args = app_from_crate!()
        .arg(
            Arg::with_name("update-repos")
                .short("u")
                .long("update")
                .help("Update all repostories")
                .conflicts_with_all(&["search", "install", "PKG"])
                .takes_value(false),
        )
        .arg(
            Arg::with_name("search")
                .short("s")
                .long("search")
                .requires("PKG")
                .conflicts_with_all(&["update-repos", "install"])
                .help("search for a package matching PKG"),
        )
        .arg(
            Arg::with_name("install")
                .short("i")
                .long("install")
                .requires("PKG")
                .conflicts_with_all(&["update-repos", "search"])
                .help("install a package PKG"),
        )
        .arg(
            Arg::with_name("PKG")
                .help("Package name. Can aldso include comparison operator and version number"),
        )
        .get_matches();

    let repos = nbpm::repos::create_repos().unwrap();
    nbpm::initialize(&repos).unwrap();

    // extract package name and version comparison parameters if some
    let (pkg_name, pkg_comp) = match cli_args.value_of("PKG") {
        Some(v) => {
            let (name, comp) = parse_pkg_info(v);
            (Some(name), comp)
        }
        None => (None, None),
    };

    // update repositories
    if cli_args.is_present("update-repos") {
        println!("[*] Updating repositories... ");
        for repo in &repos {
            println!("      {}", repo.repo_type());
            repo.update().unwrap();
        }
        println!("done!");
    }

    // search for a package
    if cli_args.is_present("search") {
        match search_pkg(&repos, (pkg_name.unwrap(), &pkg_comp)) {
            Some(matches) => utils::cli::display_pkg_list(&matches),
            None => println!("No packages found"),
        }
    }

    // install a package
    if cli_args.is_present("install") {
        let matches = match search_pkg(&repos, (pkg_name.unwrap(), &pkg_comp)) {
            Some(m) => m,
            None => exit(0),
        };

        let pkg = match matches.len() {
            0 => exit(0),
            1 => match matches.get(0) {
                Some(p) => p,
                None => unreachable!(),
            },
            _ => {
                utils::cli::display_pkg_list(&matches);
                exit(0)
            }
        };

        let to_install = match utils::resolve::resolve_dependencies(&repos, &pkg) {
            Ok(pkgs) => pkgs,
            Err(e) => {
                eprintln!("Error resolving dependencies: {:?}", e);
                exit(1);
            }
        };
        println!("The following packages are going to be installed:");
        utils::cli::display_pkg_list(&to_install);

        // ask the user for confirmation
        let mut line = String::new();
        print!(
            "Do you want to install {} {}? [N/y]",
            pkg.name(),
            pkg.version()
        );
        stdout().flush().unwrap();
        let _n = stdin()
            .read_line(&mut line)
            .expect("Cannot read user input");
        line = line.trim_end().to_string();
        if line != "Y" && line != "y" {
            println!("Installation cancelled");
            exit(0);
        }
    }
}

fn parse_pkg_info(text: &str) -> (&str, Option<(CompOp, &str)>) {
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
                    eprintln!("Missing version after comparison operator");
                    exit(0);
                }
                Some(v) => match Version::from(v) {
                    Some(_) => Some((CompOp::from_sign(operator).unwrap(), v)),
                    None => {
                        eprintln!("Unsupported version format: {}", v);
                        exit(0);
                    }
                },
            };
            break;
        }
    }
    (name, comp_ver)
}

fn search_pkg(
    repos: &[impl Repository],
    query: (&str, &Option<(CompOp, &str)>),
) -> Option<Vec<Package>> {
    let mut matches = vec![];
    for repo in repos {
        let comp = match &query.1 {
            Some(c) => Some((c.0.clone(), Version::from(c.1).unwrap())),
            None => None,
        };
        match repo.search(&[(query.0, comp)]) {
            Ok(res) => matches.extend(res[0].clone()),
            Err(e) => {
                eprintln!(
                    "Error searching package in {} repo: {:?}",
                    repo.repo_type(),
                    e
                );
                exit(1);
            }
        }
    }
    if matches.is_empty() {
        None
    } else {
        Some(matches)
    }
}

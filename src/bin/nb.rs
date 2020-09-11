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

use std::io::{self, Write};
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
        match search_pkg(&repos, pkg_name.unwrap(), &pkg_comp) {
            Some(matches) => utils::cli::display_pkg_list(&matches),
            None => println!("No packages found"),
        }
    }

    // install a package
    if cli_args.is_present("install") {
        let matches = match search_pkg(&repos, pkg_name.unwrap(), &pkg_comp) {
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

        // NOTE: RESOLVE DEPS

        println!(
            "Do you want to install {} {}? [N/y]",
            pkg.name(),
            pkg.version()
        );

        /*
        let pkgs = match search_and_display(&repos, pkg_name.unwrap(), &pkg_comp, true) {
            Some(p) => p,
            None => exit(0),
        };

        // get the id of the package to install from stdin
        println!("Select the package to install");
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
        // get the selected package and show info
        let pkg = pkgs.get(id - 1).unwrap();
        // print!("\x1B[2J\x1B[1;1H"); // clear screen
        println!("\n\n{} {}", pkg.name(), pkg.version());

        let to_install =
            pkg::utils::resolve_dependencies(&repos, &pkg).expect("Cannot resolve dependencies");
        */
    }
}

fn parse_pkg_info(text: &str) -> (&str, Option<(CompOp, Version)>) {
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
                    Some(ver) => Some((CompOp::from_sign(operator).unwrap(), ver)),
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
    pkg_name: &str,
    pkg_comp: &Option<(CompOp, Version)>,
) -> Option<Vec<Package>> {
    let mut matches = vec![];
    for repo in repos {
        match repo.search(pkg_name, &pkg_comp) {
            Ok(Some(res)) => matches.extend(res),
            Ok(None) => continue,
            Err(e) => {
                eprintln!("Error searchin in repo {}: {:?}", repo.repo_type(), e);
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

/*
fn search_and_display(
    repos: &[impl Repository],
    query: &str,
    comp_ver: &Option<(CompOp, Version)>,
    show_ids: bool,
) -> Option<Vec<Package>> {
    // search the package and make a list with packages matching the query
    let mut pkgs_list: Vec<Package> = Vec::new();
    let justify_left = CellFormat::builder().justify(Justify::Left).build();
    let mut first_row = if show_ids {
        vec![Cell::new("Id", justify_left)]
    } else {
        vec![]
    };
    first_row.push(Cell::new("repository", justify_left));
    first_row.push(Cell::new("package", justify_left));
    let mut table_contents = vec![Row::new(first_row)];
    let mut id = 1;
    for repo in repos.iter() {
        let repo_type = repo.repo_type().to_string();
        match repo.search(query, comp_ver) {
            Ok(Some(res)) => {
                for pkg in res {
                    let mut new_row = if show_ids {
                        vec![Cell::new(format!("{}", id).as_str(), justify_left)]
                    } else {
                        vec![]
                    };
                    new_row.push(Cell::new(&repo_type, justify_left));
                    new_row.push(Cell::new(
                        format!("{} {}", pkg.name(), pkg.version()).as_str(),
                        justify_left,
                    ));
                    table_contents.push(Row::new(new_row));
                    id += 1;
                    pkgs_list.push(pkg);
                }
            }
            Ok(None) => continue,
            Err(err) => eprintln!("Error searching in {} repo: {:?}", repo.repo_type(), err),
        }
    }

    if pkgs_list.is_empty() {
        return None;
    }

    let table = Table::new(table_contents, format::NO_BORDER_COLUMN_ROW)
        .expect("Failed to create results table");
    table.print_stdout().expect("Cannot diplay results table");
    Some(pkgs_list)
}
*/

#[macro_use]
extern crate clap;
extern crate regex;

use clap::Arg;
use cli_table::{
    format::{self, CellFormat, Justify},
    Cell, Row, Table,
};
use version_compare::{CompOp, Version};

use std::io::{self, Write};
use std::process::exit;

use nbpm::{pkg, Package, Repository};

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

    // extract package name, comparison operator and version if some
    let (pkg_name, comp_op, version) = match cli_args.value_of("PKG") {
        Some(query) => {
            // search for comparison operator on the query
            // NOTE: May use Regex in the future
            let mut name = None;
            let mut comp_op = None;
            let mut version = None;
            for operator in &["==", ">=", "<=", ">", "<"] {
                // if an operator is present extract the name, comparison operator and version
                if query.contains(operator) {
                    comp_op = Some(CompOp::from_sign(operator).unwrap());
                    let mut splitted = query.split(operator);
                    // extract package name
                    name = splitted.next();
                    // extract version
                    version = match splitted.next() {
                        Some("") | None => {
                            eprintln!("A version has to be provided");
                            exit(0);
                        }
                        Some(v) => match Version::from(v) {
                            Some(ver) => Some(ver),
                            None => {
                                eprintln!("Unsupported version format: {}", v);
                                exit(0);
                            }
                        },
                    };
                    break;
                }
            }
            if name.is_none() {
                name = Some(query);
            }
            (name, comp_op, version)
        }
        None => (None, None, None),
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
        search_and_display(&repos, pkg_name.unwrap(), &comp_op, &version, false);
    }

    // install a package
    if cli_args.is_present("install") {
        let pkgs = match search_and_display(&repos, pkg_name.unwrap(), &comp_op, &version, true) {
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
    }
}

fn search_and_display(
    repos: &[impl Repository],
    query: &str,
    comp_op: &Option<CompOp>,
    version: &Option<Version>,
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
        match repo.search(query, comp_op, version) {
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

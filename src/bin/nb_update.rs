// #[macro_use]
// extern crate log;
extern crate simplelog;

use std::path::Path;

// use nbpm::debian;
use nbpm::{create_repos, Repository, CONFIG};

fn main() {
    // set up environment
    // let config = Configuration::from(Path::new("config.toml")).unwrap();
    let repos = create_repos().unwrap();
    nbpm::initialize(&repos).unwrap();
    for repo in repos {
        repo.update().unwrap();
    }
    println!("[*] repositories updated");

    // debian::extract_deb("neofetch_7.1.0-1_all.deb");

    // nbpm::create_links(
    //     Path::new("/home/mike/proiektuak/lfs/nebula/nb/nbpm/test/data"),
    //     Path::new("/home/mike/proiektuak/lfs/nebula/nb/nbpm/test/dest"),
    // );
}

extern crate regex;

use nbpm::Repository;

fn main() {
    let repos = nbpm::repos::create_repos().unwrap();
    nbpm::initialize(&repos).unwrap();
    // let mut matches = vec![];
    for repo in repos {
        let match_ = repo.search("gcc", None).unwrap();
        for m in match_.unwrap() {
            println!("{}\n", m);
        }
        // matches.append(&mut match_.unwrap());
    }
}

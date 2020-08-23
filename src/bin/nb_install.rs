extern crate regex;

use regex::Regex;
use std::fs::{read_to_string, File};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

fn main() -> () {
    // let file = read_to_string("/tmp/nebula/repo/debian/Packages-main").unwrap();
    let mut buff = BufReader::new(File::open("/tmp/nebula/repo/debian/Packages-main").unwrap());

    let re = Regex::new(r"^Package: calcurse").unwrap();
    /*
    buff.lines()
        .enumerate()
        .filter(|tuple| tuple.1.is_ok()) // get line if not error
        .filter(|tuple| re.is_match(tuple.1.as_ref().unwrap().as_str())) // get matches
        .for_each(|(n, x)| {
            println!(
                "line {} seek {}: {}",
                n,
                buff.seek(SeekFrom::Current(0)).unwrap(),
                x.unwrap()
            )
        });
    */
    let mut readnext = false;
    let mut matches = vec![];
    for line in buff.lines() {
        let mut line = line.unwrap().trim_end().to_string();
        if re.is_match(&line) {
            readnext = true;
            line.push('\n');
            matches.push(line);
            continue;
        }
        if !line.is_empty() && readnext {
            println!("line: {}", line);
            if let Some(last_match) = matches.last_mut() {
                line.push('\n');
                last_match.push_str(&line);
            }
        } else if line.is_empty() {
            readnext = false;
        }
    }

    for m in matches {
        println!("-- match --");
        m.split('\n').for_each(|a| println!("line: {}", a));
    }
}

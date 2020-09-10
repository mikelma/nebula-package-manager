use std::io::{self, Write};

use crate::{NebulaError, Package};

pub fn choose_from_table(pkgs: &[Package]) -> Result<usize, NebulaError> {
    for (id, pkg) in pkgs.iter().enumerate() {
        println!(
            "[{}] {} {} (deps.: {})",
            id,
            pkg.name(),
            pkg.version(),
            match pkg.depends() {
                Some(lst) => lst.len(),
                None => 0,
            }
        );
    }
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
    Ok(id)
}

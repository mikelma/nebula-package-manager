use std::io::{self, Write};
use std::process::Command;

use crate::{NebulaError, Package};

pub fn run_cmd(cmd: &str, args: &[&str]) -> Result<(), NebulaError> {
    // create the command and add arguments if necessary
    let mut command = Command::new(cmd);
    if !args.is_empty() {
        command.args(args);
    }
    // execute command as child process
    let child = match command.output() {
        Ok(c) => c,
        Err(e) => {
            return Err(NebulaError::CmdError(format!(
                "failed to start {}: {}",
                cmd, e
            )));
        }
    };
    // read status and return result
    if child.status.success() {
        Ok(())
    } else {
        let message = String::from_utf8_lossy(&child.stderr);
        Err(NebulaError::CmdError(message.to_string()))
    }
}

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

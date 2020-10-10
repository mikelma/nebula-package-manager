use crate::{errors::*, Package, RepoType, Repository};
use version_compare::{CompOp, Version};

use std::error::Error;

pub fn search(
    queries: &[(&str, Option<(CompOp, Version)>)],
    repos: &Vec<impl Repository>,
    repo_sel: Option<RepoType>,
) -> Result<Vec<Vec<Package>>, Box<dyn Error>> {
    let mut matches = vec![vec![]; queries.len()];
    if let Some(selected) = repo_sel {
        // a repo type is selected to search for the queries
        let repo = match repos.iter().find(|r| r.repo_type() == selected) {
            Some(r) => r,
            None => {
                return Err(Box::new(NebulaError::from_msg(
                    format!("selected repository {} does not exist", selected).as_str(),
                    NbErrType::Repo,
                )))
            }
        };
        repo.search(&queries)?
            .iter()
            .enumerate()
            .for_each(|(i, m)| matches[i].extend(m.clone()));
    } else {
        // search for queries in all repos available
        for repo in repos {
            repo.search(&queries)?
                .iter()
                .enumerate()
                .for_each(|(i, m)| matches[i].extend(m.clone()));
        }
    }
    Ok(matches)
}

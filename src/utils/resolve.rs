use crate::{
    pkg::{DependsItem, DependsList},
    NebulaError, Package, Repository,
};
// use std::cell::RefCell;
use std::collections::VecDeque;
// use version_compare::Version;

pub fn resolve_dependencies(
    repos: &[impl Repository],
    package: &Package,
) -> Result<Vec<Package>, NebulaError> {
    let mut unresolved_deps: Vec<DependsItem> = Vec::new();
    // if the package contains dependencies, add them to the unresolved deps. queue
    match package.depends() {
        Some(list) => list
            .inner()
            .iter()
            .for_each(|d| unresolved_deps.push(d.clone())),
        None => return Ok(vec![package.clone()]),
    };

    let mut resolved_deps: Vec<Package> = vec![];
    loop {
        if unresolved_deps.is_empty() {
            break; // all deps. resolved
        }
        let mut opt_indxs = vec![];
        let mut queries = vec![];
        unresolved_deps.iter().for_each(|item| match item {
            DependsItem::Single(s) => queries.push((s.name(), s.version_comp())),
            DependsItem::Opts(list) => {
                opt_indxs.push(vec![]);
                list.iter().for_each(|d| {
                    let i = opt_indxs.len() - 1;
                    opt_indxs[i].push(queries.len());
                    queries.push((d.name(), d.version_comp()));
                })
            }
        });

        let mut matches = vec![vec![]; queries.len()];
        for repo in repos {
            let res = repo.search(queries.as_slice())?;
            for (i, qres) in res.iter().enumerate() {
                matches[i].extend(qres.clone());
            }
            // res.iter()
            //     .enumerate()
            //     .for_each(|(i, qr)| matches[i].extend(qr));
        }

        // check if there is any unresolved dependency
        for (i, res_vec) in matches.iter().enumerate() {
            if res_vec.is_empty() {
                return Err(NebulaError::MissingDependency(
                    unresolved_deps[i].to_string(),
                ));
            }
        }

        // resolve dependency options (DependsItem::Opts)
        /*
        for indexes in opt_indxs {
            for indx in indexes {
                matches
            }
        }
        */

        unresolved_deps.clear();

        /*
        let dep_item = unresolved_deps.pop_front().unwrap();
        match &dep_item {
            DependsItem::Single(dep) => {
                for repo in repos {
                    match repo.search(dep.name(), &dep.version_comp())? {
                        Some(p) => matches.extend(p),
                        None => continue,
                    }
                }
            }
            DependsItem::Opts(dep_list) => {
                for dep in dep_list {
                    for repo in repos {
                        match repo.search(dep.name(), &dep.version_comp())? {
                            Some(p) => matches.extend(p),
                            None => continue,
                        }
                    }
                }
            }
        */
    }
    /*
        if matches.is_empty() {
            return Err(NebulaError::MissingDependency(dep_item.to_string()));
        }
        // if there are multiple packages matching a the same dependency. The package with the
        // minimum dependencies will be chosen to fulfill the initial dependency.
        let mut min_deps_pkg = &matches[0];
        for pkg in &matches {
            if pkg.num_deps() < min_deps_pkg.num_deps() {
                min_deps_pkg = pkg;
            }
        }
        let pkg = min_deps_pkg.clone(); // selected package to fulfill the dependency
        if let Some(deps) = pkg.depends() {
            // push dependencies of the package to the unresolved dep. list
            unresolved_deps.extend(deps.inner().clone());
            // for d in deps.inner() {
            //     unresolved_deps.push_back(d.clone());
            // }
        }
        resolved_deps.push(pkg.clone());
    }
    */
    /*
    for dep_item in unresolved_deps.borrow().iter() {
        // find the package corresponding to each dependency
        let mut matches = vec![];
        match dep_item {
            // single dependency type, just search for that dep. in all repos
            DependsItem::Single(dep) => {
                for repo in repos {
                    match repo.search(dep.name(), &dep.version_comp())? {
                        Some(p) => matches.extend(p),
                        None => continue,
                    }
                }
            }
            DependsItem::Opts(dep_list) => {
                for dep in dep_list {
                    for repo in repos {
                        match repo.search(dep.name(), &dep.version_comp())? {
                            Some(p) => matches.extend(p),
                            None => continue,
                        }
                    }
                }
            }
        }
        if matches.is_empty() {
            return Err(NebulaError::MissingDependency(dep_item.to_string()));
        } else if matches.len() == 1 {
            let pkg = matches.get(0).unwrap();
            if let Some(d) = pkg.depends() {
                unresolved_deps.borrow_mut().extend(d.inner());
            }
        }
    }
    */
    Ok(resolved_deps)
}

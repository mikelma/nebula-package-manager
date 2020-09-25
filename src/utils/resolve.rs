use crate::{
    pkg::{DependsItem, DependsList},
    NebulaError, Package, Repository,
};
use petgraph::dot::{Config, Dot};
use petgraph::{graph::NodeIndex, Graph};
// use std::cell::RefCell;
// use version_compare::Version;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

pub fn resolve_dependencies(
    repos: &[impl Repository],
    package: &Package,
    save_graph: Option<&str>,
) -> Result<Vec<Package>, NebulaError> {
    let mut deps_graph = Graph::<Package, Package>::new();

    let target_pkg = deps_graph.add_node(package.clone());

    let mut unresolved_deps: HashMap<NodeIndex, Vec<DependsItem>> = HashMap::new();
    let mut edges: Vec<(NodeIndex, NodeIndex)> = vec![];

    match deps_graph[target_pkg].depends() {
        Some(list) => {
            // add dependencies of `package` to the unserolved dependencies list.
            let _ = unresolved_deps.insert(target_pkg, list.inner().clone());
        }
        None => return Ok(vec![package.clone()]),
    }

    loop {
        if unresolved_deps.is_empty() {
            break;
        }
        // prepare queries
        let mut queries = vec![];
        // an entry for each query. The item in the i-th position contains the node index
        // and the index of the dependency inside the dependencies list of the node the query
        // belongs to.
        let mut query_to_dep_map: Vec<(NodeIndex, usize)> = Vec::new();
        // for each dependency list in unresolved_deps
        for (node, dependencies) in unresolved_deps.iter() {
            // for each dependency in the dependencies list
            for (i_dep, dep) in dependencies.iter().enumerate() {
                match dep {
                    DependsItem::Single(d) => {
                        // check if any package of the graph satisfies the dependency
                        if let Some(pkg_index) = deps_graph
                            .node_indices()
                            .find(|i| deps_graph[*i].satisfies(d))
                        {
                            edges.push((*node, pkg_index));
                        } else {
                            query_to_dep_map.push((*node, i_dep));
                            queries.push((d.name(), d.version_comp()));
                        }
                    }
                    DependsItem::Opts(d_list) => {
                        for d in d_list {
                            if let Some(pkg_index) = deps_graph
                                .node_indices()
                                .find(|i| deps_graph[*i].satisfies(d))
                            {
                                edges.push((*node, pkg_index));
                                break;
                            } else {
                                query_to_dep_map.push((*node, i_dep));
                                queries.push((d.name(), d.version_comp()));
                            }
                        }
                    }
                }
            }
        }
        // println!(
        //     "-------------------- {} --------------------",
        //     queries.len()
        // );

        // get matches from queries searching for the queries in all repos
        let mut matches = vec![vec![]; queries.len()];
        for repo in repos {
            repo.search(&queries)?
                .iter()
                .enumerate()
                .for_each(|(i, m)| matches[i].extend(m.clone()));
        }
        // NOTE: `resolved` type could maybe be HashMap<NodeIndex, Vec<&DependsItem>>
        let mut resolved: HashMap<NodeIndex, Vec<DependsItem>> = HashMap::new();
        let mut new_unresolved_deps: HashMap<NodeIndex, Vec<DependsItem>> = HashMap::new();
        for (matches, (node, dep_index)) in matches.iter().zip(query_to_dep_map.iter()) {
            // println!(
            //     "Package, {}, dependency {}, mathes:",
            //     deps_graph[*node].name(),
            //     unresolved_deps[node][*dep_index],
            // );
            // matches.iter().for_each(|m| println!("      {}", m.name()));
            // println!();

            // check if the dependency was already resolved in a previous iter of this for
            if let Some(list) = resolved.get(node) {
                if list
                    .iter()
                    .find(|d| **d == unresolved_deps[node][*dep_index])
                    .is_some()
                {
                    // println!("  * Dependency already satisfied!");
                    continue;
                }
            }
            if matches.is_empty() {
                // println!("  * No matches for the query");
                continue;
            } else {
                // add the resolved dependency's package to the dependency graph
                let node_i = deps_graph.add_node(matches[0].clone());
                // add the edge from the node the dependency comes from and the dependency
                edges.push((*node, node_i));
                // add the resolved dependency to the resolved dependencies list
                if let Some(vec) = resolved.get_mut(node) {
                    vec.push(unresolved_deps[node][*dep_index].clone());
                } else {
                    resolved.insert(*node, vec![unresolved_deps[node][*dep_index].clone()]);
                }
                // add resolved dependency's dependencies to the new unserolved dependencies list
                if let Some(new_deps) = matches[0].depends() {
                    new_unresolved_deps.insert(node_i, new_deps.inner().clone());
                }
                // println!(
                //     "  * Dependency {} resolved!",
                //     unresolved_deps[node][*dep_index]
                // );
            }
        }
        unresolved_deps = new_unresolved_deps;
        // panic!();
    }
    // println!("\nedges: {:?}", edges);
    deps_graph.extend_with_edges(&edges);
    if let Some(file_name) = save_graph {
        let mut file = match File::create(file_name) {
            Ok(f) => f,
            Err(e) => return Err(NebulaError::Fs(e.to_string())),
        };
        // println!("{}", Dot::with_config(&deps_graph, &[Config::EdgeNoLabel]));
        if let Err(e) = file.write_all(
            format!("{}", Dot::with_config(&deps_graph, &[Config::EdgeNoLabel])).as_bytes(),
        ) {
            return Err(NebulaError::Io(e));
        }
    }

    // let mut space = DfsSpace::new(deps_graph);

    let sorted = match petgraph::algo::toposort(&deps_graph, None) {
        Ok(s) => s,
        Err(c) => {
            return Err(NebulaError::DependencyCicle(
                deps_graph[c.node_id()].to_string(),
            ))
        }
    };
    let mut res = vec![];
    sorted.iter().for_each(|i| res.push(deps_graph[*i].clone()));
    Ok(res)
}
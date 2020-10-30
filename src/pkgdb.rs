use globset::{Glob, GlobSetBuilder};
use serde_derive::{Deserialize, Serialize};
// use version_compare::{CompOp, Version};
use semver::Version;

use std::error::Error;
use std::fs;
use std::path::Path;

use crate::config::Arch;
use crate::pkg::{Dependency, DependsList, PkgSource};
use crate::{errors::*, repos::Query, utils, Package, RepoType};

#[derive(Serialize, Deserialize, Debug)]
pub enum Component {
    #[serde(rename = "core")]
    Core,
    #[serde(rename = "extra")]
    Extra,
}

#[derive(Serialize, Deserialize, Debug)]
struct PkgInfo {
    // mandatory fields
    name: String,
    version: String,
    // optional fields
    source: Option<String>, // if source is None, the package is a meta-package
    depends: Option<Vec<String>>,
}

impl PkgInfo {
    pub fn to_package(&self) -> Result<Package, Box<dyn Error>> {
        Ok(Package::new(
            &self.name,
            Version::parse(&self.version)?,
            PkgSource::from(RepoType::Nebula, self.source.as_deref()),
            match &self.depends {
                Some(l) => PkgInfo::deps_str_to_depitems(l)?,
                None => None,
            },
        ))
    }

    fn deps_str_to_depitems(deps_str: &Vec<String>) -> Result<Option<DependsList>, Box<dyn Error>> {
        let mut dep_list = DependsList::new();
        for dep_str in deps_str {
            if dep_str.is_empty() {
                return Err(Box::new(NebulaError::from_msg(
                    "Empty dependency while converting string to dependency",
                    NbErrType::Parsing,
                )));
            }
            let or_split: Vec<&str> = dep_str.split(" or ").collect();
            let mut opts = vec![];
            for item in or_split {
                let (name, mut ver_comp) = utils::parse_pkg_str_info(item)?;
                opts.push(Dependency::from(name, ver_comp));
            }
            if opts.is_empty() {
                return Err(Box::new(NebulaError::from_msg(
                    "Optional deps. empty while converting string to dependecies",
                    NbErrType::Parsing,
                )));
            } else if opts.len() == 1 {
                // push single dependecy
                dep_list.push(opts.pop().unwrap());
            } else {
                // push multiple options dependecy
                dep_list.push_opts(opts);
            }
        }
        if !dep_list.is_empty() {
            Ok(Some(dep_list))
        } else {
            Ok(None)
        }
    }
}

/// Database for Nebula package's
#[derive(Serialize, Deserialize, Debug)]
pub struct PkgDB {
    arch: Arch,

    #[serde(rename = "core")]
    core_info: Option<Vec<PkgInfo>>,
    #[serde(rename = "extra")]
    extra_info: Option<Vec<PkgInfo>>,

    #[serde(skip, default = "set_default_components")]
    components: Option<Vec<Component>>,
    #[serde(skip, default = "set_defaults_empty")]
    core: Option<Vec<Package>>,
    #[serde(skip, default = "set_defaults_empty")]
    extra: Option<Vec<Package>>,
}

impl PkgDB {
    pub fn from(path: &Path) -> Result<PkgDB, Box<dyn Error>> {
        // read configuration file
        match fs::read_to_string(path) {
            // deserialize configuration file
            Ok(s) => match toml::from_str::<PkgDB>(s.as_str()) {
                Ok(mut c) => {
                    let core = match c.core_info() {
                        Some(list) => Some(list.iter().map(|d| d.to_package()).collect::<Result<
                            Vec<Package>,
                            Box<dyn Error>,
                        >>(
                        )?),
                        None => None,
                    };
                    let extra = match c.extra_info() {
                        Some(list) => Some(list.iter().map(|d| d.to_package()).collect::<Result<
                            Vec<Package>,
                            Box<dyn Error>,
                        >>(
                        )?),
                        None => None,
                    };

                    let mut comps_vec = vec![];
                    if core.is_some() {
                        comps_vec.push(Component::Core);
                    }
                    if extra.is_some() {
                        comps_vec.push(Component::Extra);
                    }
                    if !comps_vec.is_empty() {
                        c.set_components(comps_vec);
                    }

                    c.set_core(core);
                    c.set_extra(extra);
                    Ok(c)
                }
                Err(e) => Err(Box::new(e)),
            },
            Err(e) => Err(Box::new(e)),
        }
    }

    pub fn arch(&self) -> &Arch {
        &self.arch
    }

    pub fn components(&self) -> Option<&[Component]> {
        self.components.as_deref()
    }

    fn core_info(&self) -> Option<&[PkgInfo]> {
        self.core_info.as_deref()
    }

    fn extra_info(&self) -> Option<&[PkgInfo]> {
        self.extra_info.as_deref()
    }

    fn set_core(&mut self, core: Option<Vec<Package>>) {
        self.core = core;
    }
    fn set_extra(&mut self, extra: Option<Vec<Package>>) {
        self.extra = extra;
    }

    fn set_components(&mut self, comps: Vec<Component>) {
        self.components = Some(comps);
    }

    pub fn search(&self, queries: &[Query]) -> Result<Vec<Vec<Package>>, Box<dyn Error>> {
        // init glob matchers from query names
        let mut builder = GlobSetBuilder::new();
        for item in queries {
            builder.add(Glob::new(item.0)?);
        }
        let glob_set = builder.build()?;

        let mut matches = vec![vec![]; queries.len()];
        for repo_component in [&self.core, &self.extra].iter() {
            // find for matches in the core repo (if available)
            if let Some(pkgs) = &repo_component {
                let mut pkg_index = 0;
                while pkg_index < pkgs.len() {
                    let mut m_indexes = glob_set.matches(pkgs[pkg_index].name());
                    if !m_indexes.is_empty() {
                        let mut i = 0;
                        while i < m_indexes.len() {
                            /*
                            if let Some((comp_op, comp_ver)) = &queries[m_indexes[i]].1 {
                                if !pkgs[i].version().compare_to(&comp_ver, &comp_op) {
                                    // if version req. is not satisfied
                                    m_indexes.remove(i); // remove match from match indexes list
                                }
                            }
                            */
                            if !queries[m_indexes[i]].1.matches(pkgs[i].version()) {
                                // if version req. is not satisfied
                                m_indexes.remove(i); // remove match from match indexes list
                            }
                            i += 1;
                        }
                        // if there are still some queries matching the package add them to the matches list
                        if !m_indexes.is_empty() {
                            for i_m in m_indexes {
                                matches[i_m].push(pkgs[pkg_index].clone());
                            }
                        }
                    }
                    pkg_index += 1;
                }
            }
        }
        Ok(matches)
    }
}

// used to set initial values of default atributtes in `Configuration`
fn set_defaults_empty() -> Option<Vec<Package>> {
    None
}
fn set_default_components() -> Option<Vec<Component>> {
    None
}

// FOR DEBUGGING ONLY
/*
#[cfg(test)]
mod test {
    use super::PkgDB;
    use std::path::Path;
    use version_compare::{CompOp, Version};

    #[test]
    fn from_file() {
        let db = PkgDB::from(Path::new("tests/pkgdb.toml")).unwrap();
        println!("{:#?}", db);
        println!("{}", toml::to_string_pretty(&db).unwrap());
        let q = vec![
            ("linux*", None),
            ("linux*", Some((CompOp::Lt, Version::from("5.0").unwrap()))),
            ("test*", None),
        ];
        let res = db.search(&q).unwrap();
        println!("search: {:#?}", res);

        assert_eq!(3, res.len());
        assert_eq!(1, res[0].len());
        assert_eq!(0, res[1].len());
        assert_eq!(1, res[2].len());
    }
}
*/

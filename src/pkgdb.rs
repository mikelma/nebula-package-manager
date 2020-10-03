use serde_derive::{Deserialize, Serialize};
use version_compare::{CompOp, Version};

use std::fs;
use std::path::Path;

use crate::config::Arch;
use crate::pkg::{Dependency, DependsItem, DependsList, PkgSource};
use crate::{utils, NebulaError, Package, RepoType};

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
    pub fn to_package(&self) -> Result<Package, NebulaError> {
        Package::new(
            &self.name,
            &self.version,
            PkgSource::from(RepoType::Nebula, self.source.as_deref()),
            match &self.depends {
                Some(l) => PkgInfo::deps_str_to_depitems(l)?,
                None => None,
            },
        )
    }

    fn deps_str_to_depitems(deps_str: &Vec<String>) -> Result<Option<DependsList>, NebulaError> {
        let mut dep_list = DependsList::new();
        for dep_str in deps_str {
            if dep_str.is_empty() {
                return Err(NebulaError::DependencyParseError);
            }
            let or_split: Vec<&str> = dep_str.split(" or ").collect();
            let mut opts = vec![];
            for item in or_split {
                let (name, ver_comp) = utils::parse_pkg_str_info(item)?;
                let dep = Dependency::from(name, ver_comp)?;
                opts.push(dep);
            }
            if opts.is_empty() {
                return Err(NebulaError::DependencyParseError);
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
    #[serde(rename = "info")]
    extra_info: Option<Vec<PkgInfo>>,

    #[serde(skip, default = "set_default_components")]
    components: Option<Vec<Component>>,
    #[serde(skip, default = "set_defaults_empty")]
    core: Option<Vec<Package>>,
    #[serde(skip, default = "set_defaults_empty")]
    extra: Option<Vec<Package>>,
}

impl PkgDB {
    pub fn from(path: &Path) -> Result<PkgDB, NebulaError> {
        // read configuration file
        match fs::read_to_string(path) {
            // deserialize configuration file
            Ok(s) => match toml::from_str::<PkgDB>(s.as_str()) {
                Ok(mut c) => {
                    let core = match c.core_info() {
                        Some(list) => Some(list.iter().map(|d| d.to_package()).collect::<Result<
                            Vec<Package>,
                            NebulaError,
                        >>(
                        )?),
                        None => None,
                    };
                    let extra = match c.extra_info() {
                        Some(list) => Some(list.iter().map(|d| d.to_package()).collect::<Result<
                            Vec<Package>,
                            NebulaError,
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
                Err(e) => Err(NebulaError::TomlDe(e)),
            },
            Err(e) => Err(NebulaError::Io(e)),
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

    // pub fn sarch(&self, queries: &[(&str, Option(CompOp, Version)]) -> Option<Vec<Package>>  {
    // }
}

// used to set initial values of default atributtes in `Configuration`
fn set_defaults_empty() -> Option<Vec<Package>> {
    None
}
fn set_default_components() -> Option<Vec<Component>> {
    None
}

// FOR DEBUGGING ONLY
#[cfg(test)]
mod test {
    use super::PkgDB;
    use std::path::Path;

    #[test]
    fn from_file() {
        let db = PkgDB::from(Path::new("tests/pkgdb.toml")).unwrap();
        println!("{:#?}", db);
        println!("{}", toml::to_string_pretty(&db).unwrap());
        // panic!();
    }
}

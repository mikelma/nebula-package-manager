use serde_derive::{Deserialize, Serialize};
// use version_compare::version::Version;
use crate::RepoType;

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Package {
    name: String,
    version: Option<String>,
    #[serde(rename = "source")]
    source: Option<PkgSource>,
    #[serde(rename = "dependencies")]
    depends: Option<Vec<Vec<Dependency>>>,
}

impl Package {
    pub fn new(name: &str) -> Package {
        Package {
            name: name.to_string(),
            version: None,
            source: None,
            depends: None,
        }
    }

    pub fn set_version(&mut self, ver: &str) {
        self.version = Some(ver.to_string());
    }

    pub fn set_source(&mut self, repo_type: RepoType, url: &str) {
        self.source = Some(PkgSource(repo_type, url.to_string()));
    }

    pub fn set_dependencies(&mut self, deps: Option<Vec<Vec<Dependency>>>) {
        self.depends = deps;
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct PkgSource(RepoType, String);

impl PkgSource {
    pub fn from(repo_type: RepoType, url: &str) -> PkgSource {
        PkgSource(repo_type, url.to_string())
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Dependency(String, String);

impl Dependency {
    /// Creates a new `Dependency` given the name and version requirement. If there is no version
    /// requirement, `version_req` parameter must be `None`.
    pub fn from(name: &str, version_req: Option<&str>) -> Dependency {
        match version_req {
            Some(v) => Dependency(name.to_string(), v.to_string()),
            None => Dependency(name.to_string(), "".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::pkg::{Dependency, Package, PkgSource};
    use crate::RepoType;
    use toml;
    #[test]
    fn package_seralization_deserialization() {
        let package = Package {
            name: "proba".to_string(),
            version: Some("1.2.3".to_string()),
            source: Some(PkgSource(
                RepoType::Nebula,
                "source.url.eus/proba".to_string(),
            )),
            depends: Some(vec![
                vec![Dependency("dep1".to_string(), "3.1".to_string())],
                vec![
                    Dependency("dep2".to_string(), "".to_string()),
                    Dependency("dep3".to_string(), "5.1".to_string()),
                ],
            ]),
        };
        let pkg_str_ser = toml::to_string(&package).unwrap();
        println!("{}", pkg_str_ser);

        let pkg_str = r#"
        name = 'proba'
        version = '1.2.3'
        source = ['nebula', 'source.url.eus/proba']
        dependencies = [[['dep1', '3.1']], [['dep2', ''], ['dep3', '5.1']]]
        "#;

        let pkg_de: Package = toml::from_str(&pkg_str).unwrap();

        assert_eq!(pkg_de, package);
    }
}

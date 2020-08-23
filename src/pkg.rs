use crate::RepoType;
use serde_derive::{Deserialize, Serialize};
// use version_compare::version::Version;

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Package {
    name: String,
    version: Option<String>,
    #[serde(rename = "repository")]
    from: Option<RepoType>,
    #[serde(rename = "dependencies")]
    depends: Option<Vec<Dependency>>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Dependency(String, String);

#[cfg(test)]
mod tests {
    use crate::pkg::{Dependency, Package};
    use crate::repos::RepoType;
    use toml;
    #[test]
    fn package_seralization_deserialization() {
        let package = Package {
            name: "proba".to_string(),
            version: Some("1.2.3".to_string()),
            from: Some(RepoType::Nebula),
            depends: Some(vec![
                Dependency("dep1".to_string(), "3.1".to_string()),
                Dependency("dep2".to_string(), "".to_string()),
            ]),
        };

        let _pkg_str_ser = toml::to_string(&package).unwrap();
        let pkg_str = r#"
        name = 'proba'
        version = '1.2.3'
        repository = 'nebula'
        dependencies = [['dep1', '3.1'], ['dep2', '']]
        "#;

        let pkg_de: Package = toml::from_str(&pkg_str).unwrap();

        assert_eq!(pkg_de, package);
    }
}

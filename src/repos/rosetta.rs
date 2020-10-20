use serde_derive::{Deserialize, Serialize};

use std::collections::HashMap;
use std::error::Error;

use crate::RepoType;

#[derive(Serialize, Deserialize, Debug)]
pub struct PkgNames {
    debian: Option<Vec<String>>,
}

impl PkgNames {
    pub fn get(&self, from: &RepoType) -> Option<&[String]> {
        match from {
            RepoType::Debian => self.debian.as_deref(),
            _ => None,
        }
    }
    pub fn contains(&self, name: &str) -> bool {
        if let Some(names) = &self.debian {
            if names.contains(&name.to_string()) {
                return true;
            }
        }
        return false;
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Rosetta {
    #[serde(flatten)]
    data: HashMap<String, PkgNames>,
}

impl Rosetta {
    pub fn new() -> Rosetta {
        Rosetta {
            data: HashMap::new(),
        }
    }
    pub fn push(&mut self, nb_name: &str, other_names: PkgNames) {
        self.data.insert(nb_name.to_string(), other_names);
    }
    pub fn name_resolve(&self, name: &str, from: &RepoType, to: &RepoType) -> Option<Vec<String>> {
        match from {
            RepoType::Nebula => Some(self.data.get(name)?.get(to)?.to_vec()),
            RepoType::Debian => {
                /*
                NOTE: Using this iterator sometimes fails to resolve the name,
                so the for version of this iterator is used instead for now
                let v: Vec<String> = self
                    .data
                    .iter()
                    .take_while(|(_, val)| val.contains(name))
                    .map(|(key, _)| key.to_owned())
                    .collect();
                */
                let mut v = vec![];
                for (key, val) in self.data.iter() {
                    if val.contains(name) {
                        v.push(key.to_string());
                    }
                }
                if v.is_empty() {
                    None
                } else {
                    Some(v)
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::RepoType;
    use toml;

    #[test]
    fn test_serialize_deserialize() {
        let rosetta_str = r#"[bar]
debian = ["test_name"]

[foo]
debian = ["egg", "foo-dev"]
"#;
        let de = toml::from_str::<Rosetta>(rosetta_str).unwrap();
        let ser = toml::to_string(&de).unwrap();

        println!("{}", rosetta_str);
        println!("{}", ser);
        // assert_eq!(rosetta_str, ser)
    }

    #[test]
    fn test_name_resolve() {
        let mut data = HashMap::new();
        data.insert(
            "bar".to_string(),
            PkgNames {
                debian: Some(vec!["test_name".to_string(), "egg".to_string()]),
            },
        );
        data.insert(
            "foo".to_string(),
            PkgNames {
                debian: Some(vec!["egg".to_string(), "foo-dev".to_string()]),
            },
        );
        let rosseta = Rosetta { data };
        println!("--------\n{:?}\n--------", rosseta);

        assert_eq!(
            vec!["egg", "foo-dev"],
            rosseta
                .name_resolve("foo", &RepoType::Nebula, &RepoType::Debian)
                .unwrap()
        );

        assert_eq!(
            vec!["bar"],
            rosseta
                .name_resolve("test_name", &RepoType::Debian, &RepoType::Nebula)
                .unwrap()
        );

        let a = rosseta
            .name_resolve("egg", &RepoType::Debian, &RepoType::Nebula)
            .unwrap();
        assert!(a == vec!["bar", "foo"] || a == vec!["foo", "bar"]);

        assert_eq!(
            None,
            rosseta.name_resolve("not_exists", &RepoType::Nebula, &RepoType::Debian)
        );
        // panic!();
    }
}

// use version_compare::version::Version;
use crate::RepoType;
use std::fmt;

#[derive(Debug)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub source: Option<PkgSource>,
    pub depends: Option<DependsList>,
}

impl Package {
    pub fn new(name: &str, version: &str) -> Package {
        Package {
            name: name.to_string(),
            version: version.to_string(),
            source: None,
            depends: None,
        }
    }
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.depends {
            Some(d) => write!(
                f,
                "Name: {}, Version: {}\nDepends ({}): {}",
                self.name,
                self.version,
                d.len(),
                d
            ),
            None => write!(f, "Name: {}, Version: {}", self.name, self.version),
        }
    }
}

#[derive(Debug)]
pub struct Dependency(String, Option<String>);

impl Dependency {
    /// Creates a new `Dependency` given the name and version requirement. If there is no version
    /// requirement, `version_req` parameter must be `None`.
    pub fn from(name: &str, version_req: Option<&str>) -> Dependency {
        match version_req {
            Some(v) => Dependency(name.to_string(), Some(v.to_string())),
            None => Dependency(name.to_string(), None),
        }
    }
}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.1 {
            Some(v) => write!(f, "{} {}", self.0, v),
            None => write!(f, "{}", self.0),
        }
    }
}

#[derive(Debug)]
enum DependsItem {
    Single(Dependency),
    Opts(Vec<Dependency>),
}

impl fmt::Display for DependsItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            DependsItem::Single(d) => write!(f, "{}", d),
            DependsItem::Opts(opts) => {
                if opts.is_empty() {
                    return write!(f, "");
                }
                for d in opts.iter().take(opts.len() - 1) {
                    write!(f, "{} or ", d)?
                }
                write!(f, "{}", opts.last().unwrap())
            }
        }
    }
}

#[derive(Debug)]
pub struct DependsList(Vec<DependsItem>);

impl DependsList {
    pub fn new() -> DependsList {
        DependsList(vec![])
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn push(&mut self, dep: Dependency) {
        // NOTE: Look if depenedencie already is in the list
        self.0.push(DependsItem::Single(dep));
    }

    pub fn push_opts(&mut self, opts: Vec<Dependency>) {
        self.0.push(DependsItem::Opts(opts));
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for DependsList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return write!(f, "");
        }
        for d in self.0.iter().take(self.0.len() - 1) {
            write!(f, "{}, ", d)?;
        }
        write!(f, "{}", self.0.last().unwrap())
    }
}

#[derive(Debug, PartialEq)]
pub struct PkgSource(RepoType, String);

impl PkgSource {
    pub fn from(repo_type: RepoType, url: &str) -> PkgSource {
        PkgSource(repo_type, url.to_string())
    }
}

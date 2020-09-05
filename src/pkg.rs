use crate::{NebulaError, RepoType};
use std::fmt;
use version_compare::{CompOp, Version};

#[derive(Debug)]
pub struct Package {
    /// The name of the package (no version included)
    name: String,
    /// Version of the package
    version: String,
    /// Source of the package
    source: Option<PkgSource>,
    /// The dependency list of the package. If it has no depenedencies, this field is None.
    depends: Option<DependsList>,
}

impl Package {
    pub fn new(
        name: &str,
        version: &str,
        source: Option<PkgSource>,
        depends: Option<DependsList>,
    ) -> Result<Package, NebulaError> {
        // check if the provided version has a compatible format with `version_compare`
        if Version::from(&version).is_none() {
            return Err(NebulaError::NotSupportedVersion);
        }

        Ok(Package {
            name: name.to_string(),
            version: version.to_string(),
            source,
            depends,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn source(&self) -> &Option<PkgSource> {
        &self.source
    }

    pub fn depends(&self) -> &Option<DependsList> {
        &self.depends
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

impl PartialEq for Package {
    fn eq(&self, other: &Self) -> bool {
        let ver_pkg = match Version::from(&self.version) {
            Some(v) => v,
            None => unreachable!(), // it is checked in the contructur of `Package`
        };
        let ver_other = match Version::from(&other.version()) {
            Some(v) => v,
            None => unreachable!(), // it is checked in the contructur of `Package`
        };

        self.name.eq(other.name()) && ver_pkg.compare(&ver_other) == CompOp::Eq
    }
}

/// Contains a package dependency. The name of the package and the version (if some) are required.
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

/// `DependsItem` objects are used as items of `DependsList`. This is useful to express different
/// dependency types, such as different package options for a dependency or an optional dependency.
#[derive(Debug)]
enum DependsItem {
    /// A single dependency. The package completly depends on this package to be present.
    Single(Dependency),
    /// Holds a vector of dependencies (different options), and only one should be installed.
    Opts(Vec<Dependency>),
    // TODO: Optional(Dependency) // optional dependencies
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

/// Defines all the dependencies a package might depend on.
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

/// Contains the information about the source of the package: which repo does the package come from
/// and the url to download the package.
#[derive(Debug, PartialEq)]
pub struct PkgSource(RepoType, String);

impl PkgSource {
    pub fn from(repo_type: RepoType, url: &str) -> PkgSource {
        PkgSource(repo_type, url.to_string())
    }
}

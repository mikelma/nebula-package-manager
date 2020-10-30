use semver::{Version, VersionReq};

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

use crate::{errors::*, RepoType};

#[derive(Debug, Clone)]
struct VersionWrapper(Version);

impl VersionWrapper {
    pub fn inner(&self) -> &Version {
        &self.0
    }
}

impl Default for VersionWrapper {
    fn default() -> Self {
        VersionWrapper(Version::new(0, 0, 0))
    }
}

#[derive(Debug, Clone, Default)]
pub struct Package {
    /// The name of the package (no version included)
    name: String,
    /// Version of the package
    version: VersionWrapper,
    /// Source of the package
    source: PkgSource,
    /// The dependency list of the package. If it has no depenedencies, this field is None.
    depends: Option<DependsList>,
}

impl Package {
    pub fn new(
        name: &str,
        version: Version,
        source: PkgSource,
        depends: Option<DependsList>,
    ) -> Package {
        Package {
            name: name.to_string(),
            version: VersionWrapper(version),
            source,
            depends,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &Version {
        self.version.inner()
    }

    pub fn source(&self) -> &PkgSource {
        &self.source
    }

    pub fn depends(&self) -> &Option<DependsList> {
        &self.depends
    }

    pub fn num_deps(&self) -> usize {
        match &self.depends {
            Some(list) => list.len(),
            None => 0,
        }
    }

    /// Returns true if the `Package` satisfies the given `Dependency`.
    pub fn satisfies(&self, dep: &Dependency) -> bool {
        /*
        if let Some((dep_comp_op, dep_ver)) = dep.version_comp() {
            // the dependency contains specific version and compare oprerator
            return self.name.eq(dep.name()) && self.version().compare_to(&dep_ver, &dep_comp_op);
        }
        // the dependency does not contain any version and comparison operator, thus the dependency
        // is satisfied if the name of the package and the name of the dependency are equal.
        self.name.eq(dep.name())
        */
        self.name.eq(dep.name()) && dep.version_req().matches(self.version.inner())
    }
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.name, self.version.inner())
    }
}

impl PartialEq for Package {
    fn eq(&self, other: &Self) -> bool {
        // two packages are considered equal if their names and versions are equal
        self.name.eq(other.name()) && *self.version.inner() == *other.version()
    }
}

#[derive(Debug, PartialEq)]
pub enum CompOp {
    Lt,
    Le,
    Eq,
    Ge,
    Gt,
}

impl FromStr for CompOp {
    type Err = NebulaError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "<" | "<<" => Ok(Self::Lt),
            "<=" => Ok(Self::Le),
            "=" | "==" => Ok(Self::Eq),
            ">=" => Ok(Self::Ge),
            ">" | ">>" => Ok(Self::Gt),
            _ => Err(NebulaError::from_msg(s, NbErrType::BadCompOp)),
        }
    }
}

/// Contains a package dependency. The name of the package, comparison operator and version are required.
#[derive(Debug, Clone, PartialEq)]
pub struct Dependency(String, VersionReq);

impl Dependency {
    /// Creates a new `Dependency` given the name and version requirement.
    pub fn from(name: &str, vercomp: VersionReq) -> Dependency {
        Dependency(name.to_string(), vercomp)
    }

    pub fn name(&self) -> &str {
        &self.0
    }

    pub fn comp_op(&self) -> CompOp {
        match CompOp::from_str(&self.1.to_string()) {
            Ok(c) => c,
            Err(e) => panic!(e),
        }
    }

    /*
    pub fn version(&self) -> Option<Version> {
        match &self.1 {
            Some((_, vs)) => match Version::from(vs) {
                Some(v) => Some(v),
                None => unreachable!(),
            },
            None => None,
        }
    }
    /// Returns all information about the version requirement of the `Dependency`. If the
    /// `Dependency` as no version requirement, `None` is returned.
    pub fn version_comp(&self) -> Option<(CompOp, Version)> {
        if let Some((comp, ver)) = &self.1 {
            let v = match Version::from(ver) {
                Some(v) => v,
                None => unreachable!(), // this is checked in the constructor
            };
            Some((comp.clone(), v))
        } else {
            None
        }
    }
    */
    pub fn version_req(&self) -> &VersionReq {
        &self.1
    }

    /// Returns true if the `Dependecy` satisfies the given `Dependency`.
    pub fn satisfies(&self, dep: &Dependency) -> bool {
        /*
        match (self.version_comp(), dep.version_comp()) {
            // compare both dependecy versions and take into account comparison operators
            (Some((_, my_ver)), Some((other_comp, other_ver))) => {
                self.0 == dep.name() && my_ver.compare_to(&other_ver, &other_comp)
            }
            // the dependency does not satisfy the given dependency if the dependency has no
            // version and the given dependency does have version a requirement
            (None, Some(_)) => false,
            // compare only dependency names if the given dependency has no version requirements
            (_, None) => self.0 == dep.name(),
        }
        */
        // compare names
        if self.0 != dep.name() {
            return false;
        }
        // compare comparison operators
        let c = dep.comp_op();
        if self.comp_op() != c {
            return false;
        }

        // check if self's version satisfies dep's requirement
        match self.1.cmp(dep.version_req()) {
            Ordering::Less => c == CompOp::Lt,
            Ordering::Equal => (c == CompOp::Le) || (c == CompOp::Eq) || (c == CompOp::Ge),
            Ordering::Greater => c == CompOp::Ge,
        }
    }
}

impl fmt::Display for Dependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        /*
        match &self.1 {
            Some((op, v)) => write!(f, "{} {:?} {}", self.0, op, v),
            None => write!(f, "{}", self.0),
        }
        */
        write!(f, "{} ({})", self.0, self.1.to_string())
    }
}

/// `DependsItem` objects are used as items of `DependsList`. This is useful to express different
/// dependency types, such as different package options for a dependency or an optional dependency.
#[derive(Debug, Clone, PartialEq)]
pub enum DependsItem {
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
#[derive(Debug, Clone)]
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

    pub fn inner(&self) -> &Vec<DependsItem> {
        &self.0
    }
    /*
    pub fn extend(&mut self, other: &DependsList) {
        self.0.extend(*other.inner());
    }
    */
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
/// and the url to download the package. If the package does not contain source url, means that the
/// package is a metapackage.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct PkgSource(RepoType, Option<String>);

impl PkgSource {
    pub fn from(repo_type: RepoType, url: Option<&str>) -> PkgSource {
        let url = match url {
            Some(u) => Some(u.to_string()),
            None => None,
        };
        PkgSource(repo_type, url)
    }

    pub fn repo_type(&self) -> &RepoType {
        &self.0
    }

    pub fn source_url(&self) -> Option<&str> {
        if let Some(s) = &self.1 {
            Some(s.as_str())
        } else {
            None
        }
    }

    /// Returns true when the package is a matepackage
    pub fn is_meta(&self) -> bool {
        self.1.is_none()
    }
}

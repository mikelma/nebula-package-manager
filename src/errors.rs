use std::error::Error;
use std::fmt;

#[derive(Debug, Clone)]
pub enum NbErrType {
    // repos
    Repo,
    // files and io
    HashCheck,
    // parsing
    Parsing,
    // package
    PackageNotFound,
    // version
    // VersionComp,
    VersionFmt,
    VersionNotFound,
    BadCompOp,
    // dependecy
    DependencyNotFound,
    DependencyCicle,
    // command
    Cmd,
    // File system errors
    CannotRemoveBadLinks,
}

impl fmt::Display for NbErrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            NbErrType::Repo => write!(f, "Repository error"),
            NbErrType::HashCheck => write!(f, "Hash check error"),
            NbErrType::PackageNotFound => write!(f, "Package not found"),
            NbErrType::Parsing => write!(f, "Error while parsing"),
            // NbErrType::VersionComp => write!(f, "Verision comparison error"),
            NbErrType::VersionFmt => write!(f, "Incompatible version format"),
            NbErrType::VersionNotFound => write!(f, "Version not found"),
            NbErrType::BadCompOp => write!(f, "Incorrect or bad comparison operator"),
            NbErrType::DependencyNotFound => write!(f, "Dependency not found"),
            NbErrType::DependencyCicle => write!(f, "Dependency cycle found"),
            NbErrType::Cmd => write!(f, "Command error"),
            NbErrType::CannotRemoveBadLinks => write!(
                f,
                "Cannot remove bad links, links have to be manually removed"
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NebulaError {
    err_type: NbErrType,
    message: Option<String>,
}

impl NebulaError {
    /// Creates a new `NebulaError` of the given error type, with no error message
    pub fn new(err_type: NbErrType) -> NebulaError {
        NebulaError {
            message: None,
            err_type,
        }
    }

    /// Creates a new `NebulaError` of the given error type containing an error message
    pub fn from_msg(msg: &str, err_type: NbErrType) -> NebulaError {
        NebulaError {
            message: Some(msg.to_string()),
            err_type,
        }
    }
}

impl fmt::Display for NebulaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.message {
            Some(m) => write!(f, "{}:{}", self.err_type, m),
            None => write!(f, "{}", self.err_type),
        }
    }
}

impl Error for NebulaError {}

/*
#[derive(Debug)]
pub enum NebulaError_old {
    // Io(std::io::Error),
    // TomlDe(toml::de::Error),
    RepoConfigNotFound,
    IncorrectHash,
    /// Command execution error
    CmdError(String),
    /// File system related error
    Fs(String),
    DependencyParseError,
    VersionParsingError,
    SourceParsingError,
    /// The package version format is unsupported
    NotSupportedVersion,
    MissingDependency(String),
    PackageNotFound(&'static str),
    MissingVersion,
    MissingCompOp,
    GlobError(String),
    DependencyCicle(String),
}

impl fmt::Display for NebulaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {}
    }
}

impl Error for NebulaError {}
*/

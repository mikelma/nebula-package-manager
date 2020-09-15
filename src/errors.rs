#[derive(Debug)]
pub enum NebulaError {
    Io(std::io::Error),
    TomlDe(toml::de::Error),
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
}

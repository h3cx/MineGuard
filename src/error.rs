use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("Undefined error")]
    Generic,
}

#[derive(Debug, Clone, Error)]
pub enum VersionError {
    #[error("Incorrect major version: {0}")]
    IncorrectMajor(String),

    #[error("Incorrect minor version: {0}")]
    IncorrectMinor(String),

    #[error("Incorrect patch version: {0}")]
    IncorrectPatch(String),

    #[error("Incorrect major version: {0}")]
    IncorrectYear(String),

    #[error("Incorrect minor version: {0}")]
    IncorrectWeek(String),

    #[error("Incorrect patch version: {0}")]
    IncorrectBuild(String),

    #[error("Missing major version")]
    MissingMajor,

    #[error("Missing minor version")]
    MissingMinor,

    #[error("Missing patch version")]
    MissingPatch,

    #[error("Invalid snapshot format")]
    InvalidSnapshotFormat,

    #[error("Too many components")]
    ExtraComponents,

    #[error("Unrecognized version format: {0}")]
    UnknownVersionFormat(String),
}

#[derive(Debug, Clone, Error)]
pub enum HandleError {
    #[error("Invalid Minecraft Version: {0}")]
    InvalidVersion(String),

    #[error("Invalid server root directory: {0}")]
    InvalidDirectory(String),

    #[error("Invalid relative JAR path: {0}")]
    InvalidPathJAR(String),
}

#[derive(Debug, Clone, Error)]
pub enum SubscribeError {
    #[error("No stdout found")]
    NoStdout,

    #[error("No stderr found")]
    NoStderr,
}

#[derive(Debug, Clone, Error)]
pub enum ServerError {
    #[error("Server is already running")]
    AlreadyRunning,

    #[error("Server is not running")]
    NotRunning,

    #[error("Server crashed early")]
    EarlyCrash,

    #[error("Failed to run java command")]
    CommandFailed,

    #[error("Failed to access child stdout pipe")]
    NoStdoutPipe,

    #[error("Failed to access child stdin pipe")]
    NoStdinPipe,

    #[error("Failed to access child stderr pipe")]
    NoStderrPipe,

    #[error("Failed to write to stdin")]
    StdinWriteFailed,

    #[error("Failed to open eula.txt")]
    NoEULA,
    #[error("Failed to write eula.txt")]
    WriteEULAFailed,
}

#[cfg(feature = "events")]
#[derive(Debug, Clone, Error)]
pub enum ParserError {
    #[error("ParserError")]
    ParserError,
}

#[derive(Debug, Clone, Error)]
pub enum CreationError {
    #[error("CreationError")]
    CreationError,

    #[error("Invalid directory")]
    DirectoryError,

    #[error("Failed to parse manifest")]
    ManifestError,

    #[error("Version does not exist")]
    VersionError,

    #[error("Network Error")]
    NetworkError,
}
#[derive(Debug, Clone, Error)]
pub enum ManifestError {
    #[error("ManifestError")]
    ManifestError,

    #[error("Failed to load mainfest")]
    LoadUrlError,
    #[error("Failed to parse manifest json")]
    JsonParseError,
}

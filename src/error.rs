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

type Result<T> = std::result::Result<T, Error>;

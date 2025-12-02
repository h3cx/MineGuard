use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("Undefined error")]
    Generic,
}

type Result<T> = std::result::Result<T, Error>;

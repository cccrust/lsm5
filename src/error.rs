use std::io;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Corruption(String),
    KeyNotFound,
    WalReplayError(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {}", e),
            Error::Corruption(msg) => write!(f, "Data corruption: {}", msg),
            Error::KeyNotFound => write!(f, "Key not found"),
            Error::WalReplayError(msg) => write!(f, "WAL replay error: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
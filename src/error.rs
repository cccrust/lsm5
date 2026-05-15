use std::io;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Corruption(String),
    KeyNotFound,
    WalReplayError(String),
    InvalidConfig(String),
    FileFormatError(String),
    CompactionError(String),
    TransactionError(String),
    NoActiveTransaction,
    TransactionAlreadyCommitted,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {}", e),
            Error::Corruption(msg) => write!(f, "Data corruption: {}", msg),
            Error::KeyNotFound => write!(f, "Key not found"),
            Error::WalReplayError(msg) => write!(f, "WAL replay error: {}", msg),
            Error::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            Error::FileFormatError(msg) => write!(f, "File format error: {}", msg),
            Error::CompactionError(msg) => write!(f, "Compaction error: {}", msg),
            Error::TransactionError(msg) => write!(f, "Transaction error: {}", msg),
            Error::NoActiveTransaction => write!(f, "No active transaction"),
            Error::TransactionAlreadyCommitted => write!(f, "Transaction already committed"),
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

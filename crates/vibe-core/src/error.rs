//! The error type every `vibe-core` operation returns.
use std::fmt;
use std::path::PathBuf;

/// Errors returned by `vibe-core`. Binaries map them to exit codes.
#[derive(Debug)]
pub enum Error {
    /// An I/O operation on `path` failed.
    Io {
        /// The path the operation touched.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// The caller passed an argument the operation cannot accept.
    Invalid(String),
}

/// `Result` with [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Error::Invalid(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::Invalid(_) => None,
        }
    }
}

use std::time::Duration;

/// Result type used by dockerlet.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by dockerlet.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The local Docker daemon is unavailable or rejected the Unix
    /// socket connection.
    #[error("docker daemon unavailable: {0}")]
    DaemonUnavailable(String),
    /// Startup did not complete before the configured timeout.
    #[error("startup timed out after {0:?}")]
    StartupTimeout(Duration),
    /// A readiness probe failed.
    #[error("readiness probe failed: {0}")]
    ReadinessFailed(String),
    /// Bollard returned an internal error.
    #[error("internal bollard error: {0}")]
    Bollard(String),
    /// dockerlet hit an internal invariant or host error.
    #[error("internal: {0}")]
    Internal(String),
}

impl From<bollard::errors::Error> for Error {
    fn from(value: bollard::errors::Error) -> Self {
        match value {
            bollard::errors::Error::SocketNotFoundError(path) => {
                Self::DaemonUnavailable(format!("Docker socket not found: {path}"))
            }
            other => Self::Bollard(other.to_string()),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Internal(value.to_string())
    }
}

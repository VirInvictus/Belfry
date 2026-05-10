//! Error types for `belfry-core`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("worker channel closed")]
    WorkerChannelClosed,

    #[error("library path not configured and could not be determined from XDG environment")]
    NoLibraryPath,

    #[error("invalid enum value: {field} = {value:?}")]
    InvalidEnum { field: &'static str, value: String },
}

pub type Result<T> = std::result::Result<T, Error>;

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for Error {
    fn from(_: tokio::sync::mpsc::error::SendError<T>) -> Self {
        Self::WorkerChannelClosed
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for Error {
    fn from(_: tokio::sync::oneshot::error::RecvError) -> Self {
        Self::WorkerChannelClosed
    }
}

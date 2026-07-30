//! Every Tauri command returns `Result<T, AppError>`. Serializes to
//! `{"kind": "...", "message": "..."}` — see `frontend/src/lib/api.ts`'s
//! `errorMessage` for how the frontend turns that into toast copy.
//!
//! The kind set is deliberately small and maps to what the app can
//! actually fail at (not every internal error type gets its own kind):
//! a missing id, a network-dependent fetch, a database/pool problem, or
//! everything else (capture pipeline internals, filesystem, malformed
//! input, task-join failures).
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum AppError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Network(String),
    #[error("{0}")]
    Database(String),
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn not_found(what: &str) -> Self {
        AppError::NotFound(format!("{what} not found"))
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Database(e.to_string())
    }
}

impl From<r2d2::Error> for AppError {
    fn from(e: r2d2::Error) -> Self {
        AppError::Database(e.to_string())
    }
}

impl From<tokio::task::JoinError> for AppError {
    fn from(e: tokio::task::JoinError) -> Self {
        AppError::Internal(e.to_string())
    }
}

impl From<crate::capture::CaptureError> for AppError {
    fn from(e: crate::capture::CaptureError) -> Self {
        match &e {
            crate::capture::CaptureError::Render(_) => AppError::Network(e.to_string()),
            _ => AppError::Internal(e.to_string()),
        }
    }
}

impl From<crate::sources::rss::RssSyncError> for AppError {
    fn from(e: crate::sources::rss::RssSyncError) -> Self {
        use crate::sources::rss::RssSyncError;
        match &e {
            RssSyncError::Fetch(_) | RssSyncError::Parse(_) => AppError::Network(e.to_string()),
            RssSyncError::Db(_) | RssSyncError::Pool(_) => AppError::Database(e.to_string()),
        }
    }
}

impl From<crate::sources::direct_link::DirectLinkError> for AppError {
    fn from(e: crate::sources::direct_link::DirectLinkError) -> Self {
        use crate::sources::direct_link::DirectLinkError;
        match e {
            DirectLinkError::Capture(inner) => inner.into(),
            DirectLinkError::Db(inner) => inner.into(),
            DirectLinkError::Pool(inner) => inner.into(),
        }
    }
}

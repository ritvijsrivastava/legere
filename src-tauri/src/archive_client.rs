//! Talks to the self-hosted `legere-server` archive server over HTTP:
//! submitting a capture job, polling its status, and downloading the
//! finished `.zim`. Every function here takes the `reqwest::Client` it
//! needs directly rather than the whole `AppState` — a deliberately
//! unguarded client, separate from `AppState.http_client`'s SSRF guard
//! (see that field's own docs for why), since this one only ever talks to
//! one fixed, user-configured, trusted endpoint.
//!
//! Nothing here awaits its own result inline from a command handler —
//! [`spawn_submission`] is fire-and-forget from the capture call sites,
//! and [`get_job_status`]/[`download_archive_to`] are driven by the
//! archive reconciler's own poll loop.

use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

use crate::db::DbPool;

#[derive(Debug, Error)]
pub enum ArchiveClientError {
    #[error("archive server is not configured (empty base URL)")]
    NotConfigured,
    #[error("request to archive server failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("archive server returned status {0}")]
    Status(reqwest::StatusCode),
    #[error("archive job for this article was never submitted")]
    NotFound,
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Deserialize)]
pub struct ArchiveStatusResponse {
    pub status: String,
    pub zim_main_path: Option<String>,
    pub error: Option<String>,
}

fn base_url(configured_url: &str) -> Result<String, ArchiveClientError> {
    if configured_url.is_empty() {
        return Err(ArchiveClientError::NotConfigured);
    }
    Ok(configured_url.trim_end_matches('/').to_string())
}

/// `PUT /v1/archives/{id}` — idempotent: the server no-ops if a job for
/// `id` is already pending/running/ready, so a caller never needs to check
/// first.
pub async fn submit_capture_job(
    client: &reqwest::Client,
    server_url: &str,
    token: &str,
    id: &str,
    url: &str,
) -> Result<(), ArchiveClientError> {
    let base = base_url(server_url)?;
    let response = client
        .put(format!("{base}/v1/archives/{id}"))
        .bearer_auth(token)
        .json(&serde_json::json!({ "url": url }))
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(ArchiveClientError::Status(response.status()));
    }
    Ok(())
}

/// `GET /v1/archives/{id}/status`. A `404` (job never submitted — e.g. the
/// app closed between `capture_local` returning and the fire-and-forget
/// `PUT` completing) surfaces as [`ArchiveClientError::NotFound`], which
/// the reconciler treats as "re-submit," not a hard failure.
pub async fn get_job_status(
    client: &reqwest::Client,
    server_url: &str,
    token: &str,
    id: &str,
) -> Result<ArchiveStatusResponse, ArchiveClientError> {
    let base = base_url(server_url)?;
    let response = client
        .get(format!("{base}/v1/archives/{id}/status"))
        .bearer_auth(token)
        .send()
        .await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(ArchiveClientError::NotFound);
    }
    if !response.status().is_success() {
        return Err(ArchiveClientError::Status(response.status()));
    }
    Ok(response.json().await?)
}

/// `GET /v1/archives/{id}/file` — streams the finished `.zim` straight to
/// `dest` via a temp-file-then-rename, so a reader that opens `dest`
/// mid-download (e.g. a stale cached path lookup racing this write) never
/// sees a truncated file.
pub async fn download_archive_to(
    client: &reqwest::Client,
    server_url: &str,
    token: &str,
    id: &str,
    dest: &Path,
) -> Result<(), ArchiveClientError> {
    let base = base_url(server_url)?;
    let response = client
        .get(format!("{base}/v1/archives/{id}/file"))
        .bearer_auth(token)
        .send()
        .await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(ArchiveClientError::NotFound);
    }
    if !response.status().is_success() {
        return Err(ArchiveClientError::Status(response.status()));
    }
    let bytes = response.bytes().await?;

    let tmp_path = dest.with_extension("zim.tmp");
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&tmp_path, &bytes).await?;
    tokio::fs::rename(&tmp_path, dest).await?;
    Ok(())
}

/// Fire-and-forget: submits `final_url`'s capture job right after
/// `capture_local` returns, never awaited by the calling command/sync
/// loop. `pool`/`client` are owned clones (both cheap — `r2d2::Pool` and
/// `reqwest::Client` are both `Arc`-backed) so this can run fully detached
/// via `tauri::async_runtime::spawn` with no borrow tying it to the
/// caller's stack frame.
///
/// A missing/unconfigured server, or any request failure, is logged and
/// dropped — the archive reconciler's own poll loop is what actually
/// notices a submission never landed (the server has no record for the
/// id) and retries it, so this fire-and-forget call never needs its own
/// retry logic.
pub fn spawn_submission(pool: DbPool, client: reqwest::Client, id: String, final_url: String) {
    tauri::async_runtime::spawn(async move {
        let (server_url, token) = {
            let settings = tokio::task::spawn_blocking(move || {
                let conn = pool.get().map_err(|e| e.to_string())?;
                crate::db::queries::get_settings(&conn).map_err(|e| e.to_string())
            })
            .await;
            match settings {
                Ok(Ok(settings)) => (settings.archive_server_url, settings.archive_server_token),
                Ok(Err(err)) => {
                    tracing::warn!(%id, %err, "archive submission: failed to read settings, skipping");
                    return;
                }
                Err(err) => {
                    tracing::warn!(%id, %err, "archive submission: settings task panicked, skipping");
                    return;
                }
            }
        };

        if let Err(err) = submit_capture_job(&client, &server_url, &token, &id, &final_url).await {
            tracing::warn!(%id, %err, "archive submission failed, will be retried by the reconciler");
        }
    });
}

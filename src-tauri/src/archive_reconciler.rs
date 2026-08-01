//! Reconciles each `server`-sourced article's `archive_status` against
//! `legere-server`, and holds the shared download+LRU-cache logic used
//! both by [`spawn`]'s own poll loop and by
//! `commands::articles::open_for_reading`'s download-if-needed spawn.
//!
//! Runs unconditionally once the app starts — unlike RSS `autosync`,
//! archive completion isn't something a user would ever want to disable;
//! it just sits idle (an early return each tick) until a server URL is
//! configured in settings.

use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::archive_client::{self, ArchiveClientError};
use crate::db::queries;
use crate::error::AppError;
use crate::events;
use crate::state::AppState;

/// How many `server`-sourced articles may hold a locally cached full
/// archive at once, across every article currently in `reading` state.
/// The least-recently-opened one beyond this is evicted (bytes removed,
/// row/`reading_state`/server copy untouched) — see
/// [`queries::enforce_archive_lru`].
pub const LOCAL_ARCHIVE_CACHE_CAP: u32 = 5;

const RECONCILE_INTERVAL: Duration = Duration::from_secs(2 * 60);

pub fn spawn(app: AppHandle) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(RECONCILE_INTERVAL);
        loop {
            interval.tick().await;
            let state = app.state::<AppState>();
            reconcile_once(&app, &state).await;
        }
    })
}

async fn configured_server(state: &AppState) -> Option<(String, String)> {
    let pool = state.pool.clone();
    let settings = tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| e.to_string())?;
        queries::get_settings(&conn).map_err(|e| e.to_string())
    })
    .await;
    match settings {
        Ok(Ok(settings)) if !settings.archive_server_url.is_empty() => {
            Some((settings.archive_server_url, settings.archive_server_token))
        }
        _ => None,
    }
}

async fn reconcile_once(app: &AppHandle, state: &AppState) {
    let Some((server_url, token)) = configured_server(state).await else {
        return;
    };

    let pending = {
        let pool = state.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let conn = pool.get().map_err(|e| e.to_string())?;
            queries::list_pending_archive_jobs(&conn).map_err(|e| e.to_string())
        })
        .await;
        match result {
            Ok(Ok(rows)) => rows,
            Ok(Err(err)) => {
                tracing::warn!(%err, "archive reconciler: failed to list pending jobs, skipping tick");
                return;
            }
            Err(err) => {
                tracing::warn!(%err, "archive reconciler: db task panicked, skipping tick");
                return;
            }
        }
    };

    for (id, link) in pending {
        reconcile_one(app, state, &server_url, &token, &id, &link).await;
    }
}

async fn reconcile_one(
    app: &AppHandle,
    state: &AppState,
    server_url: &str,
    token: &str,
    id: &str,
    link: &str,
) {
    match archive_client::get_job_status(&state.server_http_client, server_url, token, id).await {
        Ok(status) => match status.status.as_str() {
            "ready" => {
                let Some(zim_main_path) = status.zim_main_path else {
                    tracing::warn!(%id, "archive reconciler: server reported ready with no zim_main_path");
                    return;
                };
                let pool = state.pool.clone();
                let (id_owned, zim_main_path_owned) = (id.to_string(), zim_main_path);
                let updated = tokio::task::spawn_blocking(move || {
                    let conn = pool.get()?;
                    Ok::<_, AppError>(queries::set_archive_ready(
                        &conn,
                        &id_owned,
                        &zim_main_path_owned,
                    )?)
                })
                .await;
                if let Err(err) = updated {
                    tracing::warn!(%id, %err, "archive reconciler: db task panicked recording ready status");
                    return;
                } else if let Ok(Err(err)) = updated {
                    tracing::warn!(%id, %err, "archive reconciler: failed to record ready status");
                    return;
                }

                let currently_reading = {
                    let pool = state.pool.clone();
                    let id_owned = id.to_string();
                    tokio::task::spawn_blocking(move || {
                        let conn = pool.get()?;
                        Ok::<_, AppError>(queries::get_article(&conn, &id_owned)?)
                    })
                    .await
                };
                let is_reading = matches!(
                    currently_reading,
                    Ok(Ok(Some(detail))) if detail.reading_state == "reading"
                );
                if is_reading {
                    download_and_cache_archive(app, state, server_url, token, id).await;
                }
            }
            "failed" => {
                let error = status.error.unwrap_or_else(|| "capture failed".to_string());
                let pool = state.pool.clone();
                let (id_owned, error_owned) = (id.to_string(), error);
                let _ = tokio::task::spawn_blocking(move || {
                    let conn = pool.get()?;
                    Ok::<_, AppError>(queries::set_archive_failed(&conn, &id_owned, &error_owned)?)
                })
                .await;
            }
            // "pending"/"running" — nothing to do yet, next tick checks again.
            _ => {}
        },
        Err(ArchiveClientError::NotFound) => {
            if let Err(err) =
                archive_client::submit_capture_job(&state.server_http_client, server_url, token, id, link)
                    .await
            {
                tracing::warn!(%id, %err, "archive reconciler: re-submission failed, will retry next tick");
            }
        }
        Err(err) => {
            tracing::warn!(%id, %err, "archive reconciler: status check failed, will retry next tick");
        }
    }
}

/// Downloads `id`'s full archive to `archives/<id>.zim` and records it as
/// cached, then enforces [`LOCAL_ARCHIVE_CACHE_CAP`] — evicting whichever
/// `server`-sourced article(s) fall out (bytes removed, `ZimCache` entry
/// dropped; row/`reading_state`/server copy untouched). Emits
/// `archive:ready` for `id` on success so the reader's Original tab can
/// unlock without a manual refetch.
///
/// Shared by the reconciler (once a pending job reports ready for a
/// currently-`reading` article) and `commands::articles::open_for_reading`
/// (once the reader opens an article whose archive is already `ready` but
/// not yet cached locally).
pub async fn download_and_cache_archive(
    app: &AppHandle,
    state: &AppState,
    server_url: &str,
    token: &str,
    id: &str,
) {
    let rel_path = format!("archives/{id}.zim");
    let dest = state.data_dir.join(&rel_path);
    if let Err(err) =
        archive_client::download_archive_to(&state.server_http_client, server_url, token, id, &dest)
            .await
    {
        tracing::warn!(%id, %err, "archive download failed, will retry on next reconcile tick");
        return;
    }

    let pool = state.pool.clone();
    let (id_owned, rel_path_owned) = (id.to_string(), rel_path);
    let evicted = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        queries::set_archive_local_path(&conn, &id_owned, Some(&rel_path_owned))?;
        Ok::<_, AppError>(queries::enforce_archive_lru(&conn, LOCAL_ARCHIVE_CACHE_CAP)?)
    })
    .await;

    state.zim_cache.evict(id);
    events::emit_archive_ready(app, id);
    events::emit_articles_changed(app);

    if let Ok(Ok(evicted_files)) = evicted {
        for (evicted_id, evicted_path) in evicted_files {
            let _ = tokio::fs::remove_file(state.data_dir.join(&evicted_path)).await;
            state.zim_cache.evict(&evicted_id);
        }
    }
}

/// Fire-and-forget wrapper: reads settings itself, then downloads if
/// configured. Used by `open_for_reading`, which must return the article
/// detail immediately rather than waiting on a (potentially slow)
/// download.
pub fn spawn_download_if_ready(app: AppHandle, id: String) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let Some((server_url, token)) = configured_server(&state).await else {
            return;
        };
        download_and_cache_archive(&app, &state, &server_url, &token, &id).await;
    });
}

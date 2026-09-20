//! Wires `remote_sync::engine` into the app: loads this device's bucket
//! config, runs one sync pass, records success/failure, and emits the
//! `remote-sync:*` events the frontend listens for. Mirrors `sync.rs`'s
//! `sync_all_sources`/`spawn_autosync` shape for RSS sources, kept as a
//! separate module/event namespace since the two sync systems are
//! independent (see `events::emit_remote_sync_started`'s doc comment).

use std::path::PathBuf;
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::db::{sync_config, DbPool};
use crate::error::AppError;
use crate::events::{self, RemoteSyncFinished, RemoteSyncProgress};
use crate::remote_sync::engine::{self, SyncOutcome};
use crate::state::AppState;

/// Runs one sync pass if (and only if) sync is configured and enabled on
/// this device. `Ok(None)` means "nothing to do" (never configured, or
/// disabled) — not an error, since both the hourly scheduler and a
/// disabled-by-choice device hit this constantly and it's not a failure.
pub async fn run_remote_sync_once(
    app: &AppHandle,
    state: &AppState,
) -> Result<Option<SyncOutcome>, AppError> {
    let pool = state.pool.clone();
    let config = {
        let conn = pool.get()?;
        sync_config::get_remote_sync_config(&conn)?
    };
    let Some(config) = config else {
        return Ok(None);
    };
    if !config.enabled {
        return Ok(None);
    }

    events::emit_remote_sync_started(app);

    let outcome = run_once_inner(pool.clone(), state.data_dir.clone(), &config, app.clone()).await;

    match outcome {
        Ok(outcome) => {
            let conn = pool.get()?;
            sync_config::record_sync_success(&conn)?;
            events::emit_remote_sync_finished(
                app,
                &RemoteSyncFinished {
                    pulled: outcome.pulled,
                    pushed: outcome.pushed,
                    tombstones_applied: outcome.tombstones_applied,
                },
            );
            if outcome.pulled > 0 || outcome.tombstones_applied > 0 {
                events::emit_articles_changed(app);
                events::emit_source_changed(app);
            }
            Ok(Some(outcome))
        }
        Err(err) => {
            let app_err: AppError = err;
            if let Ok(conn) = pool.get() {
                let _ = sync_config::record_sync_failure(&conn, &app_err.to_string());
            }
            events::emit_remote_sync_error(app, &app_err.to_string());
            Err(app_err)
        }
    }
}

/// The part of a sync pass that actually needs both the DB connection and
/// the network client together: run on a dedicated blocking thread via
/// `spawn_blocking`, driven to completion in-place with `block_on` rather
/// than `tokio::spawn`, since `rusqlite::Connection` isn't `Sync` and a
/// future holding `&Connection` across an `.await` (which
/// `engine::run_sync` does throughout — every pull/push interleaves a DB
/// read/write with a network call) is therefore not `Send`. `block_on`
/// only requires the *closure* to be `Send + 'static`, not the future it
/// drives, which is exactly what's needed here.
async fn run_once_inner(
    pool: DbPool,
    data_dir: PathBuf,
    config: &sync_config::RemoteSyncConfig,
    app: AppHandle,
) -> Result<SyncOutcome, AppError> {
    let client = engine::client_from_config(config)?;
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let on_progress = move |progress: engine::SyncProgress| {
            events::emit_remote_sync_progress(
                &app,
                &RemoteSyncProgress {
                    phase: progress.phase.as_str().to_string(),
                    completed: progress.completed,
                    total: progress.total,
                },
            );
        };
        tauri::async_runtime::block_on(engine::run_sync(
            &conn,
            &client,
            &data_dir,
            chrono::Utc::now(),
            on_progress,
        ))
        .map_err(AppError::from)
    })
    .await?
}

/// Spawns the foreground hourly cross-device sync loop, mirroring
/// `sync::spawn_autosync`'s shape (immediate run, then every `interval`).
/// A disabled/unconfigured device just no-ops every tick via
/// `run_remote_sync_once` rather than this loop checking first — cheaper
/// to keep one code path than to duplicate the "is sync on?" check here
/// too.
pub fn spawn_remote_sync_autosync(
    app: AppHandle,
    interval: Duration,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            let state = app.state::<AppState>();
            let _ = run_remote_sync_once(&app, &state).await;
        }
    })
}

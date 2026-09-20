//! Wires `remote_sync::engine` into the app: loads this device's bucket
//! config, runs one sync pass, records success/failure, and emits the
//! `remote-sync:*` events the frontend listens for. Mirrors `sync.rs`'s
//! `sync_all_sources`/`spawn_autosync` shape for RSS sources, kept as a
//! separate module/event namespace since the two sync systems are
//! independent (see `events::emit_remote_sync_started`'s doc comment).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::db::{DbPool, sync_config};
use crate::error::AppError;
use crate::events::{self, RemoteSyncFinished, RemoteSyncProgress};
use crate::remote_sync::engine::{self, SyncOutcome};
use crate::state::AppState;

/// `run_once_inner` needs to distinguish a cancelled pass (silent, not an
/// error) from every other failure, which is lost the moment anything
/// gets converted to the flattened `AppError` — so this stays as
/// `engine::SyncError` (or the one other error source, the DB pool)
/// until `run_remote_sync_once` has had a chance to check for
/// `Cancelled` specifically.
enum RunOnceError {
    Sync(engine::SyncError),
    Pool(r2d2::Error),
}

impl From<r2d2::Error> for RunOnceError {
    fn from(e: r2d2::Error) -> Self {
        Self::Pool(e)
    }
}

impl From<engine::SyncError> for RunOnceError {
    fn from(e: engine::SyncError) -> Self {
        Self::Sync(e)
    }
}

impl From<RunOnceError> for AppError {
    fn from(e: RunOnceError) -> Self {
        match e {
            RunOnceError::Sync(e) => AppError::from(e),
            RunOnceError::Pool(e) => AppError::from(e),
        }
    }
}

/// Runs one sync pass if (and only if) sync is configured and enabled on
/// this device, and no other pass is already running on it. `Ok(None)`
/// means "nothing happened" — never configured, disabled, a pass was
/// already in progress, or this one was cancelled mid-flight — none of
/// which are errors: the hourly scheduler and a disabled-by-choice
/// device hit the first two constantly, a scheduler tick racing a manual
/// "Sync now" hits the third, and turning sync off mid-pass
/// (`commands::remote_sync::save_remote_sync_config`) hits the fourth.
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

    let cancel = {
        let mut guard = state.remote_sync_cancel.lock().await;
        if guard.is_some() {
            // Another pass (scheduled or manual) is already running on
            // this device — refuse rather than let two run at once.
            return Ok(None);
        }
        let flag = Arc::new(AtomicBool::new(false));
        *guard = Some(flag.clone());
        flag
    };

    events::emit_remote_sync_started(app);

    let outcome = run_once_inner(pool.clone(), state.data_dir.clone(), &config, app.clone(), cancel).await;
    *state.remote_sync_cancel.lock().await = None;

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
        Err(RunOnceError::Sync(engine::SyncError::Cancelled)) => {
            // Deliberately not recorded as `last_error` and not toasted
            // — see this function's doc comment.
            events::emit_remote_sync_cancelled(app);
            Ok(None)
        }
        Err(err) => {
            let app_err: AppError = err.into();
            if let Ok(conn) = pool.get() {
                let _ = sync_config::record_sync_failure(&conn, &app_err.to_string());
            }
            events::emit_remote_sync_error(app, &app_err.to_string());
            Err(app_err)
        }
    }
}

/// Flips the cancellation flag for whatever sync pass is currently
/// running on this device, if any — a no-op (not an error) if nothing
/// is running, since the "Cancel" button and the auto-cancel-on-disable
/// path (`commands::remote_sync::save_remote_sync_config`) both call
/// this without first checking whether a pass actually happens to be in
/// flight at that exact moment.
pub async fn cancel_running_sync(state: &AppState) {
    if let Some(flag) = state.remote_sync_cancel.lock().await.as_ref() {
        flag.store(true, Ordering::Relaxed);
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
    cancel: Arc<AtomicBool>,
) -> Result<SyncOutcome, RunOnceError> {
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
            &cancel,
            on_progress,
        ))
        .map_err(RunOnceError::Sync)
    })
    .await
    .map_err(|join_err| RunOnceError::Sync(engine::SyncError::TaskJoin(join_err.to_string())))?
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

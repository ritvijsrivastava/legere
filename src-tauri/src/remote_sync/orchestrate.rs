//! Wires `remote_sync::engine` into the app: loads this device's bucket
//! config, runs one sync pass, records success/failure, and emits the
//! `remote-sync:*` events the frontend listens for. Mirrors `sync.rs`'s
//! `sync_all_sources`/`spawn_autosync` shape for RSS sources, kept as a
//! separate module/event namespace since the two sync systems are
//! independent (see `events::emit_remote_sync_started`'s doc comment).

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

    let outcome = run_once_inner_state(state, pool.clone(), &config, app.clone(), cancel).await;
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

/// Stops the currently-running background sync loop (if any), safely —
/// used wherever the loop needs to be torn down and possibly replaced
/// with a fresh one (`commands::remote_sync::save_remote_sync_config`,
/// on every config save, not just on/off toggles).
///
/// Does *not* just `JoinHandle::abort()` the loop outright: that's a
/// hard kill at whatever `.await` point the task happens to be at, which
/// — if a pass was mid-flight — skips `run_remote_sync_once`'s own
/// cleanup (`*state.remote_sync_cancel.lock().await = None`) entirely.
/// That leaves the cancel guard stuck `Some` forever, which then makes
/// every future sync attempt (scheduled or manual) silently refuse to
/// start, permanently, for the rest of the app session — this was a real
/// bug, not a hypothetical one. Instead: cooperatively cancel
/// (`cancel_running_sync`) so an in-flight pass exits at its own next
/// checkpoint and runs its normal cleanup, wait for that to actually
/// happen (polling `remote_sync_cancel` back to `None`), and only then
/// abort the now-idle loop task (safe: it's parked at `ticker.tick()`,
/// holding nothing).
pub async fn stop_remote_sync_loop(state: &AppState) {
    cancel_running_sync(state).await;
    while state.remote_sync_cancel.lock().await.is_some() {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    if let Some(handle) = state.remote_sync_handle.lock().await.take() {
        handle.abort();
    }
}

/// Reuses `state.remote_sync_client_cache`'s `S3Client` (and the
/// `reqwest::Client`/connection pool underneath it) if it was already built
/// from this exact `config`, rather than paying a fresh connection setup
/// (and, over HTTPS, a fresh TLS handshake) on every call — `S3Client::new`
/// itself is cheap (no I/O), but the `reqwest::Client` it wraps is not,
/// since a brand-new one starts with an empty connection pool. This
/// matters most for `ensure_article_images_synced`, called on every
/// `open_for_reading`, far more often than a sync pass runs. A config
/// change (any field, including just re-saving the same bucket) naturally
/// invalidates the cache since it no longer compares equal — no separate
/// invalidation path needed.
pub async fn cached_client(
    state: &AppState,
    config: &sync_config::RemoteSyncConfig,
) -> Result<crate::remote_sync::client::S3Client, engine::SyncError> {
    let mut guard = state.remote_sync_client_cache.lock().await;
    if let Some((cached_config, client)) = guard.as_ref()
        && cached_config == config
    {
        return Ok(client.clone());
    }
    let client = engine::client_from_config(config)?;
    *guard = Some((config.clone(), client.clone()));
    Ok(client)
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
async fn run_once_inner_state(
    state: &AppState,
    pool: DbPool,
    config: &sync_config::RemoteSyncConfig,
    app: AppHandle,
    cancel: Arc<AtomicBool>,
) -> Result<SyncOutcome, RunOnceError> {
    let client = cached_client(state, config).await?;
    let data_dir = state.data_dir.clone();
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

/// Fetches `id`'s images from the bucket if (and only if) sync is
/// configured/enabled on this device and the article doesn't already
/// have them locally — see `lazy_images::pull_article_images`'s doc
/// comment for the full "why lazy" rationale. Called from
/// `commands::articles::open_for_reading`; deliberately infallible
/// (logs and swallows every error) since a sync/network hiccup must
/// never block opening an article that's otherwise perfectly readable.
pub async fn ensure_article_images_synced(state: &AppState, id: &str) {
    let pool = state.pool.clone();
    let config = {
        let Ok(conn) = pool.get() else { return };
        sync_config::get_remote_sync_config(&conn).ok().flatten()
    };
    let Some(config) = config else {
        return;
    };
    if !config.enabled {
        return;
    }

    let hero_image_path = {
        let Ok(conn) = pool.get() else { return };
        match crate::db::sync_rows::get_article_for_sync(&conn, id) {
            Ok(Some(row)) => row.hero_image_path,
            _ => return,
        }
    };

    let client = match cached_client(state, &config).await {
        Ok(client) => client,
        Err(err) => {
            tracing::warn!(%err, "could not build sync client for lazy image fetch");
            return;
        }
    };

    crate::remote_sync::lazy_images::pull_article_images(
        &client,
        &state.data_dir,
        id,
        hero_image_path.as_deref(),
    )
    .await;
}

/// Spawns the foreground cross-device sync loop, mirroring
/// `sync::spawn_autosync`'s shape. `run_immediately` controls whether the
/// very first tick fires right away (`tokio::time::interval`'s default —
/// appropriate at app startup and when sync is freshly turned on) or is
/// skipped so the first sync only happens after a full `interval` has
/// elapsed (appropriate when this is just a fresh loop replacing an
/// already-running one, e.g. `sync_interval_hours` changed, or other
/// config fields were edited while sync was already enabled — neither of
/// those is a reason to force an extra sync pass right now). A
/// disabled/unconfigured device just no-ops every tick via
/// `run_remote_sync_once` rather than this loop checking first — cheaper
/// to keep one code path than to duplicate the "is sync on?" check here
/// too.
pub fn spawn_remote_sync_autosync(
    app: AppHandle,
    interval: Duration,
    run_immediately: bool,
) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        if !run_immediately {
            // Consumes `tokio::time::interval`'s always-immediate first
            // tick so the loop instead waits a full `interval` before its
            // first sync.
            ticker.tick().await;
        }
        loop {
            ticker.tick().await;
            let state = app.state::<AppState>();
            let _ = run_remote_sync_once(&app, &state).await;
        }
    })
}

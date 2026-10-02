//! Tauri commands for cross-device sync setup and the manual "Sync now"
//! action. See `remote_sync::orchestrate` for the actual sync pass and
//! `db::sync_config` for how bucket credentials/status are stored.

use std::time::Duration;

use tauri::{AppHandle, State};

use crate::db::sync_config::{self, RemoteSyncConfig, RemoteSyncStatus};
use crate::error::AppError;
use crate::remote_sync::engine::{self, SyncOutcome};
use crate::remote_sync::orchestrate::{
    cancel_running_sync, run_remote_sync_once, spawn_remote_sync_autosync, stop_remote_sync_loop,
};
use crate::state::AppState;

/// `None` if sync has never been configured on this device.
#[tauri::command]
pub async fn get_remote_sync_config(
    state: State<'_, AppState>,
) -> Result<Option<RemoteSyncConfig>, AppError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok(sync_config::get_remote_sync_config(&conn)?)
    })
    .await?
}

#[tauri::command]
pub async fn get_remote_sync_status(
    state: State<'_, AppState>,
) -> Result<RemoteSyncStatus, AppError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok(sync_config::get_remote_sync_status(&conn)?)
    })
    .await?
}

/// Verifies the provider honors conditional writes without persisting
/// anything — backs a settings-screen "Test connection" action, run
/// before the user commits to enabling sync against a given bucket. See
/// `remote_sync::client::S3Client::probe_conditional_write_support`'s doc
/// comment for why an unsupported provider is rejected outright rather
/// than degraded to a weaker fallback.
#[tauri::command]
pub async fn test_remote_sync_connection(config: RemoteSyncConfig) -> Result<bool, AppError> {
    let client = engine::client_from_config(&config)?;
    Ok(client.probe_conditional_write_support().await?)
}

/// Saves the bucket configuration as given, trusting
/// `config.conditional_writes_verified` rather than re-probing the
/// bucket here -- that network round-trip (four requests against the
/// user's own bucket, see `S3Client::probe_conditional_write_support`)
/// used to run on *every* save, including ones with nothing to do with
/// connectivity at all (e.g. just the sync-frequency stepper), which
/// made every such save visibly take several seconds. The dedicated
/// `test_remote_sync_connection` command (the settings screen's
/// "Test connection" button) is the only place that probe runs now --
/// `RemoteSyncDialog`'s `draftConfig` sets `conditional_writes_verified`
/// from that button's own last result, and resets it the moment any
/// bucket-identifying field is edited afterward so a stale "ok" can't
/// survive an edit unverified.
///
/// Starts or stops the sync loop to match the (possibly just-changed)
/// `enabled` flag, always restarting it (rather than only on enable) so a
/// changed `sync_interval_hours` takes effect on the loop immediately --
/// mirrors `commands::settings::update_settings`'s handling of RSS
/// autosync. Restarting never forces an extra sync pass right now unless
/// this save is what actually turns sync on (see `spawn_remote_sync_autosync`'s
/// `run_immediately` -- editing the interval or other config fields while
/// already enabled just reschedules the existing cadence), and never hard-
/// kills a pass that happens to be mid-flight at save time (see
/// `stop_remote_sync_loop`).
#[tauri::command]
pub async fn save_remote_sync_config(
    app: AppHandle,
    state: State<'_, AppState>,
    config: RemoteSyncConfig,
) -> Result<RemoteSyncConfig, AppError> {
    let pool = state.pool.clone();
    let config_clone = config.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok::<_, AppError>(sync_config::save_remote_sync_config(&conn, &config_clone)?)
    })
    .await??;

    let was_running = state.remote_sync_handle.lock().await.is_some();
    // Turning sync off (or just replacing the loop below) must not leave a
    // pass silently finishing -- or failing to write a manifest anyone can
    // still see -- in the background after this save has already
    // returned; see `stop_remote_sync_loop`'s doc comment for why this is
    // a cooperative stop-and-wait, not a hard abort.
    stop_remote_sync_loop(&state).await;
    if config.enabled {
        let interval = Duration::from_secs(config.sync_interval_hours as u64 * 60 * 60);
        let handle = spawn_remote_sync_autosync(app, interval, !was_running);
        *state.remote_sync_handle.lock().await = Some(handle);
    }

    Ok(config)
}

/// Cancels whatever cross-device sync pass is currently running on this
/// device, if any — a no-op otherwise. Backs the settings screen's
/// "Cancel" action, shown only while a sync is in progress.
#[tauri::command]
pub async fn cancel_remote_sync(state: State<'_, AppState>) -> Result<(), AppError> {
    cancel_running_sync(&state).await;
    Ok(())
}

/// The manual "Sync now" button. `Ok(None)` means sync isn't configured
/// or is currently disabled - a UI state the settings screen should
/// already prevent this button from being reachable in, but handled
/// gracefully rather than assumed unreachable.
#[tauri::command]
pub async fn remote_sync_now(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<SyncOutcome>, AppError> {
    run_remote_sync_once(&app, &state).await
}

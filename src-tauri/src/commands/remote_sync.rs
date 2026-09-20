//! Tauri commands for cross-device sync setup and the manual "Sync now"
//! action. See `remote_sync::orchestrate` for the actual sync pass and
//! `db::sync_config` for how bucket credentials/status are stored.

use std::time::Duration;

use tauri::{AppHandle, State};

use crate::db::sync_config::{self, RemoteSyncConfig, RemoteSyncStatus};
use crate::error::AppError;
use crate::remote_sync::engine::{self, SyncOutcome};
use crate::remote_sync::orchestrate::{cancel_running_sync, run_remote_sync_once, spawn_remote_sync_autosync};
use crate::state::AppState;

const REMOTE_SYNC_INTERVAL: Duration = Duration::from_secs(60 * 60);

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

/// Saves the bucket configuration. If `config.enabled` is `true`, this
/// re-verifies conditional-write support first and refuses to save (and
/// refuses to enable) against a provider that doesn't support it, rather
/// than silently persisting a config that would only fail later during a
/// scheduled sync. Saving with `enabled: false` (e.g. just entering
/// credentials without turning sync on yet) skips that check.
///
/// Starts or stops the hourly sync loop to match the (possibly
/// just-changed) `enabled` flag, mirroring
/// `commands::settings::update_settings`'s handling of RSS autosync.
#[tauri::command]
pub async fn save_remote_sync_config(
    app: AppHandle,
    state: State<'_, AppState>,
    mut config: RemoteSyncConfig,
) -> Result<RemoteSyncConfig, AppError> {
    if config.enabled {
        let client = engine::client_from_config(&config)?;
        let verified = client.probe_conditional_write_support().await?;
        if !verified {
            return Err(AppError::Internal(
                "This storage provider doesn't support conditional writes, which cross-device sync requires to avoid two devices silently overwriting each other. Try a different provider or bucket.".to_string(),
            ));
        }
        config.conditional_writes_verified = true;
    }

    let pool = state.pool.clone();
    let config_clone = config.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok::<_, AppError>(sync_config::save_remote_sync_config(&conn, &config_clone)?)
    })
    .await??;

    let mut handle_guard = state.remote_sync_handle.lock().await;
    if config.enabled {
        if handle_guard.is_none() {
            *handle_guard = Some(spawn_remote_sync_autosync(app, REMOTE_SYNC_INTERVAL));
        }
    } else {
        if let Some(handle) = handle_guard.take() {
            handle.abort();
        }
        // Turning sync off must not leave a pass silently finishing (or
        // failing to write a manifest anyone can still see) in the
        // background after the user thinks it's off.
        cancel_running_sync(&state).await;
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

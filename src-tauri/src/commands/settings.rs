use std::time::Duration;

use tauri::{AppHandle, State};

use crate::db::queries;
use crate::error::AppError;
use crate::models::Settings;
use crate::state::AppState;
use crate::sync::spawn_autosync;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, AppError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok(queries::get_settings(&conn)?)
    })
    .await?
}

/// Persists `settings` and restarts the foreground autosync task to match
/// the (possibly just-changed) `autosync` flag and/or
/// `autosync_interval_hours` -- always re-spawned from scratch (so a
/// changed interval always takes effect on the loop) rather than left
/// running on a stale one, but never forcing an extra sync of every
/// source right now unless this save is what actually turns autosync on
/// -- see `spawn_autosync`'s `run_immediately`. Saving any other setting
/// (theme, font size, ...) while autosync is already on must not
/// re-trigger a full source sync as a side effect; only actually turning
/// autosync on does that.
///
/// RSS fetches have no equivalent of cross-device sync's stuck-guard
/// failure mode (see `remote_sync::orchestrate::stop_remote_sync_loop`'s
/// doc comment), so a plain abort here -- rather than a cooperative
/// stop-and-wait -- stays safe.
#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<(), AppError> {
    let pool = state.pool.clone();
    let settings_clone = settings.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok::<_, AppError>(queries::update_settings(&conn, &settings_clone)?)
    })
    .await??;

    let mut handle_guard = state.autosync_handle.lock().await;
    let was_running = handle_guard.is_some();
    if let Some(handle) = handle_guard.take() {
        handle.abort();
    }
    if settings.autosync {
        let interval = Duration::from_secs(settings.autosync_interval_hours as u64 * 60 * 60);
        *handle_guard = Some(spawn_autosync(app, interval, !was_running));
    }

    Ok(())
}

use std::time::Duration;

use tauri::{AppHandle, State};

use crate::db::queries;
use crate::error::AppError;
use crate::models::Settings;
use crate::state::AppState;
use crate::sync::spawn_autosync;

const AUTOSYNC_INTERVAL: Duration = Duration::from_secs(15 * 60);

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, AppError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok(queries::get_settings(&conn)?)
    })
    .await?
}

/// Persists `settings` and starts/stops the foreground autosync task to
/// match the (possibly just-changed) `autosync` flag.
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
    if settings.autosync {
        if handle_guard.is_none() {
            *handle_guard = Some(spawn_autosync(app, AUTOSYNC_INTERVAL));
        }
    } else if let Some(handle) = handle_guard.take() {
        handle.abort();
    }

    Ok(())
}

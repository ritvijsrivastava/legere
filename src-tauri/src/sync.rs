use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::db::queries;
use crate::sources::rss;
use crate::state::AppState;

/// Syncs every active RSS source once. Foreground-only: this is called both
/// by the manual "sync all" command and by the autosync interval task below,
/// which only runs while the app process is alive (no Android WorkManager
/// integration in the MVP).
pub async fn sync_all_sources(state: &AppState) -> u32 {
    let sources = {
        let conn = match state.pool.get() {
            Ok(c) => c,
            Err(err) => {
                tracing::error!(%err, "failed to get db connection for sync");
                return 0;
            }
        };
        match queries::list_sources(&conn) {
            Ok(s) => s,
            Err(err) => {
                tracing::error!(%err, "failed to list sources for sync");
                return 0;
            }
        }
    };

    let mut total_new = 0u32;
    for source in sources {
        if source.source_type != "rss" || source.status != "active" {
            continue;
        }
        match rss::sync_rss_source(state, &source).await {
            Ok(count) => {
                total_new += count;
                if let Ok(conn) = state.pool.get() {
                    let _ = queries::mark_source_synced(&conn, &source.id);
                }
            }
            Err(err) => {
                tracing::warn!(source_id = %source.id, error = %err, "RSS sync failed");
                if let Ok(conn) = state.pool.get() {
                    let _ = queries::mark_source_error(&conn, &source.id, &err.to_string());
                }
            }
        }
    }
    total_new
}

/// Spawns the foreground autosync loop: syncs immediately, then every
/// `interval`. Stopping it is just aborting the returned handle (see
/// `commands::settings::update_settings`).
///
/// Uses `tauri::async_runtime::spawn` rather than `tokio::spawn` directly —
/// this is called from `Builder::setup`, which runs before any `tokio`
/// reactor is entered on that thread; only Tauri's own runtime handle is
/// guaranteed available there.
pub fn spawn_autosync(app: AppHandle, interval: Duration) -> tauri::async_runtime::JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            let state = app.state::<AppState>();
            sync_all_sources(&state).await;
        }
    })
}

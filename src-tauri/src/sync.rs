use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::db::queries;
use crate::events::{self, SyncError, SyncFinished};
use crate::sources::rss;
use crate::state::AppState;

/// Syncs every RSS source once — `active` sources normally, and `error`
/// sources too (retried every cycle, no backoff for MVP: this is what
/// actually lets an errored source recover on its own instead of being
/// stuck until the user manually intervenes). `paused` sources are
/// skipped. Foreground-only: this is called both by the manual "sync all"
/// command and by the autosync interval task below, which only runs
/// while the app process is alive (no Android WorkManager integration in
/// the MVP).
pub async fn sync_all_sources(app: &AppHandle, state: &AppState) -> u32 {
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

    let syncable: Vec<_> = sources
        .into_iter()
        .filter(|s| s.source_type == "rss" && matches!(s.status.as_str(), "active" | "error"))
        .collect();
    if syncable.is_empty() {
        return 0;
    }

    events::emit_sync_started(app, "all");

    let mut total_new = 0u32;
    let mut errors = Vec::new();
    for source in syncable {
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
                errors.push(SyncError {
                    source_id: source.id.clone(),
                    message: err.to_string(),
                });
            }
        }
    }

    events::emit_sync_finished(
        app,
        &SyncFinished {
            new_article_count: total_new,
            errors,
        },
    );
    events::emit_source_changed(app);
    if total_new > 0 {
        events::emit_articles_changed(app);
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
            sync_all_sources(&app, &state).await;
        }
    })
}

use tauri::State;

use crate::db::queries;
use crate::models::{Source, SyncResult};
use crate::sources::rss;
use crate::state::AppState;
use crate::sync::sync_all_sources;

#[tauri::command]
pub async fn list_sources(state: State<'_, AppState>) -> Result<Vec<Source>, String> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| e.to_string())?;
        queries::list_sources(&conn).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Registers a recurring RSS source and triggers its first sync immediately
/// (rather than leaving it empty until the next autosync interval, up to 15
/// minutes away). Direct-link "sources" are a distinct one-shot capture,
/// not a recurring source — see `commands::articles::add_direct_link_article`.
#[tauri::command]
pub async fn add_source(
    state: State<'_, AppState>,
    source_type: String,
    value: String,
) -> Result<Source, String> {
    match source_type.as_str() {
        "rss" => {
            let pool = state.pool.clone();
            let feed_url = value;
            let name = feed_url.clone();
            let source = tokio::task::spawn_blocking(move || {
                let conn = pool.get().map_err(|e| e.to_string())?;
                queries::insert_rss_source(&conn, &name, &feed_url).map_err(|e| e.to_string())
            })
            .await
            .map_err(|e| e.to_string())??;

            if let Err(err) = sync_source_by_id(&state, &source.id).await {
                tracing::warn!(source_id = %source.id, %err, "initial sync after add_source failed");
            }

            let pool = state.pool.clone();
            let id = source.id.clone();
            tokio::task::spawn_blocking(move || {
                let conn = pool.get().map_err(|e| e.to_string())?;
                queries::get_source(&conn, &id)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| "source not found".to_string())
            })
            .await
            .map_err(|e| e.to_string())?
        }
        other => Err(format!("unknown source type: {other}")),
    }
}

#[tauri::command]
pub async fn toggle_source_pause(state: State<'_, AppState>, id: String) -> Result<Source, String> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| e.to_string())?;
        queries::toggle_source_pause(&conn, &id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn remove_source(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| e.to_string())?;
        queries::remove_source(&conn, &id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Syncs one source by id, marking it synced (or errored) afterward. Shared
/// by the `sync_source` command and `add_source`'s immediate first sync.
async fn sync_source_by_id(state: &AppState, id: &str) -> Result<u32, String> {
    let source = {
        let pool = state.pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get().map_err(|e| e.to_string())?;
            queries::get_source(&conn, &id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "source not found".to_string())
        })
        .await
        .map_err(|e| e.to_string())??
    };

    let result = rss::sync_rss_source(state, &source).await;

    let pool = state.pool.clone();
    let id = id.to_string();
    let sync_ok = result.is_ok();
    let error_message = result.as_ref().err().map(|e| e.to_string());
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| e.to_string())?;
        if sync_ok {
            queries::mark_source_synced(&conn, &id).map_err(|e| e.to_string())
        } else {
            queries::mark_source_error(&conn, &id, error_message.as_deref().unwrap_or("sync failed"))
                .map_err(|e| e.to_string())
        }
    })
    .await
    .map_err(|e| e.to_string())??;

    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_source(state: State<'_, AppState>, id: String) -> Result<SyncResult, String> {
    let new_article_count = sync_source_by_id(&state, &id).await?;
    Ok(SyncResult { new_article_count })
}

#[tauri::command]
pub async fn sync_all(state: State<'_, AppState>) -> Result<SyncResult, String> {
    let new_article_count = sync_all_sources(&state).await;
    Ok(SyncResult { new_article_count })
}

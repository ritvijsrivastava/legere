use tauri::{AppHandle, State};

use crate::capture;
use crate::db::queries;
use crate::events;
use crate::models::{ArticleDetail, ArticleSummary};
use crate::state::AppState;

#[tauri::command]
pub async fn list_articles(state: State<'_, AppState>) -> Result<Vec<ArticleSummary>, String> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| e.to_string())?;
        queries::list_articles(&conn).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_article(
    state: State<'_, AppState>,
    id: String,
) -> Result<ArticleDetail, String> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| e.to_string())?;
        queries::get_article(&conn, &id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "article not found".to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mark_read(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| e.to_string())?;
        queries::mark_read(&conn, &id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn toggle_favorite(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| e.to_string())?;
        queries::toggle_favorite(&conn, &id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Called on a debounce from the reader's scroll handler.
#[tauri::command]
pub async fn save_reading_progress(
    state: State<'_, AppState>,
    id: String,
    progress: f64,
) -> Result<(), String> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| e.to_string())?;
        queries::save_reading_progress(&conn, &id, progress).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Deletes an article and its files (ZIM archive + hero thumbnail, if
/// any). Deleting an id that no longer exists is treated as success —
/// idempotent, so a double-click or a stale UI state can't surface an
/// error for something that's already gone.
#[tauri::command]
pub async fn delete_article(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let pool = state.pool.clone();
    let id_for_query = id.clone();
    let deleted = tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| e.to_string())?;
        queries::delete_article(&conn, &id_for_query).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    if let Some(files) = deleted {
        let _ = tokio::fs::remove_file(state.data_dir.join(&files.zim_path)).await;
        if let Some(hero) = &files.hero_image_path {
            let _ = tokio::fs::remove_file(state.data_dir.join(hero)).await;
        }
        state.zim_cache.evict(&id);
        events::emit_articles_changed(&app);
    }

    Ok(())
}

/// Re-runs the capture pipeline for an existing article against its
/// already-stored link, overwriting its content in place (same id, same
/// ZIM/hero-image file paths). Mainly for articles captured before this
/// remodel, whose ZIM archives predate self-contained localization and
/// whose readable view may still reference remote images.
#[tauri::command]
pub async fn recapture_article(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<ArticleDetail, String> {
    let existing = {
        let pool = state.pool.clone();
        let id = id.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get().map_err(|e| e.to_string())?;
            queries::get_article(&conn, &id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "article not found".to_string())
        })
        .await
        .map_err(|e| e.to_string())??
    };

    let output = capture::capture_article(&state.http_client, &state.data_dir, &id, &existing.link)
        .await
        .map_err(|e| e.to_string())?;

    let pool = state.pool.clone();
    let id_for_update = id.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| e.to_string())?;
        queries::update_captured_article(&conn, &id_for_update, &output).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    // The ZIM at this article's path was just overwritten — a cached
    // reader from before the recapture must not keep serving it.
    state.zim_cache.evict(&id);
    events::emit_articles_changed(&app);

    let pool = state.pool.clone();
    let id_for_fetch = id.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| e.to_string())?;
        queries::get_article(&conn, &id_for_fetch)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "article not found after recapture".to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn add_direct_link_article(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
) -> Result<ArticleSummary, String> {
    let result = crate::sources::direct_link::capture_direct_link(&state, &url)
        .await
        .map_err(|e| e.to_string());
    if result.is_ok() {
        events::emit_articles_changed(&app);
    }
    result
}

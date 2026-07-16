use tauri::State;

use crate::db::queries;
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

#[tauri::command]
pub async fn add_direct_link_article(
    state: State<'_, AppState>,
    url: String,
) -> Result<ArticleSummary, String> {
    crate::sources::direct_link::capture_direct_link(&state, &url)
        .await
        .map_err(|e| e.to_string())
}

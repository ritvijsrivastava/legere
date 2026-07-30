use tauri::{AppHandle, State};

use crate::capture;
use crate::db::queries;
use crate::error::AppError;
use crate::events;
use crate::models::{ArticleDetail, ArticleSummary};
use crate::state::AppState;

#[tauri::command]
pub async fn list_articles(state: State<'_, AppState>) -> Result<Vec<ArticleSummary>, AppError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok(queries::list_articles(&conn)?)
    })
    .await?
}

#[tauri::command]
pub async fn get_article(
    state: State<'_, AppState>,
    id: String,
) -> Result<ArticleDetail, AppError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        queries::get_article(&conn, &id)?.ok_or_else(|| AppError::not_found("article"))
    })
    .await?
}

#[tauri::command]
pub async fn mark_read(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok(queries::mark_read(&conn, &id)?)
    })
    .await?
}

#[tauri::command]
pub async fn toggle_favorite(state: State<'_, AppState>, id: String) -> Result<bool, AppError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok(queries::toggle_favorite(&conn, &id)?)
    })
    .await?
}

/// Called on a debounce from the reader's scroll handler.
#[tauri::command]
pub async fn save_reading_progress(
    state: State<'_, AppState>,
    id: String,
    progress: f64,
) -> Result<(), AppError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok(queries::save_reading_progress(&conn, &id, progress)?)
    })
    .await?
}

/// Deletes an article and its files (ZIM archive + hero thumbnail, if
/// any). Deleting an id that no longer exists is treated as success —
/// idempotent, so a double-click or a stale UI state can't surface an
/// error for something that's already gone.
#[tauri::command]
pub async fn delete_article(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let pool = state.pool.clone();
    let id_for_query = id.clone();
    let deleted = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok::<_, AppError>(queries::delete_article(&conn, &id_for_query)?)
    })
    .await??;

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
) -> Result<ArticleDetail, AppError> {
    let existing = {
        let pool = state.pool.clone();
        let id = id.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::get_article(&conn, &id)?.ok_or_else(|| AppError::not_found("article"))
        })
        .await??
    };

    let output = capture::capture_article(
        capture::render::Renderer::Static(&state.http_client),
        &state.http_client,
        &state.data_dir,
        &id,
        &existing.link,
    )
    .await?;

    let pool = state.pool.clone();
    let id_for_update = id.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok::<_, AppError>(queries::update_captured_article(
            &conn,
            &id_for_update,
            &output,
        )?)
    })
    .await??;

    // The ZIM at this article's path was just overwritten — a cached
    // reader from before the recapture must not keep serving it.
    state.zim_cache.evict(&id);
    events::emit_articles_changed(&app);

    let pool = state.pool.clone();
    let id_for_fetch = id.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        queries::get_article(&conn, &id_for_fetch)?
            .ok_or_else(|| AppError::not_found("article (after recapture)"))
    })
    .await?
}

#[tauri::command]
pub async fn add_direct_link_article(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
) -> Result<ArticleSummary, AppError> {
    let result = crate::sources::direct_link::capture_direct_link(&state, &url)
        .await
        .map_err(AppError::from);
    if result.is_ok() {
        events::emit_articles_changed(&app);
    }
    result
}

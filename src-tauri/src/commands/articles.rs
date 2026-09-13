use tauri::{AppHandle, State};

use crate::capture;
use crate::db::queries;
use crate::error::AppError;
use crate::events;
use crate::models::{ArticleDetail, ArticlePage, ArticlePageRequest, ArticleSummary};
use crate::state::AppState;

/// Keyset-paginated article listing backing the library/favorites views
/// (see `ArticlePageRequest`/`queries::list_articles_page` for the
/// pagination and filter semantics) — replaces the old `list_articles`,
/// which fetched and returned the entire table on every call and stopped
/// scaling once a library reached hundreds/thousands of articles.
#[tauri::command]
pub async fn list_articles_page(
    state: State<'_, AppState>,
    request: ArticlePageRequest,
) -> Result<ArticlePage, AppError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let cursor = match (&request.cursor_fetched_at, &request.cursor_id) {
            (Some(fetched_at), Some(id)) => Some((fetched_at.as_str(), id.as_str())),
            _ => None,
        };
        let search = request
            .search
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let result = queries::list_articles_page(
            &conn,
            &queries::ArticlePageQuery {
                cursor,
                limit: request.limit.max(1),
                search,
                source_name: request.source_name.as_deref(),
                tags: &request.tags,
                favorited_only: request.favorited_only,
            },
        )?;
        Ok(ArticlePage {
            items: result.items,
            has_more: result.has_more,
            next_cursor: result.next_cursor,
        })
    })
    .await?
}

#[tauri::command]
pub async fn count_all_articles(state: State<'_, AppState>) -> Result<i64, AppError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok(queries::count_all_articles(&conn)?)
    })
    .await?
}

#[tauri::command]
pub async fn count_unread(state: State<'_, AppState>) -> Result<i64, AppError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok(queries::count_unread(&conn)?)
    })
    .await?
}

#[tauri::command]
pub async fn count_favorited(state: State<'_, AppState>) -> Result<i64, AppError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok(queries::count_favorited(&conn)?)
    })
    .await?
}

#[tauri::command]
pub async fn list_categories(state: State<'_, AppState>) -> Result<Vec<(String, i64)>, AppError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok(queries::list_categories(&conn)?)
    })
    .await?
}

#[tauri::command]
pub async fn list_tags(state: State<'_, AppState>) -> Result<Vec<(String, i64)>, AppError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok(queries::list_tags(&conn)?)
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

/// Transitions an article into `reading` — called when the reader opens
/// it, whether it was previously `unread` or `read` (reopening a finished
/// article resumes it).
#[tauri::command]
pub async fn open_for_reading(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<ArticleDetail, AppError> {
    let pool = state.pool.clone();
    let id_for_transition = id.clone();
    let detail = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        queries::transition_to_reading(&conn, &id_for_transition)?;
        queries::get_article(&conn, &id_for_transition)?
            .ok_or_else(|| AppError::not_found("article"))
    })
    .await??;

    events::emit_articles_changed(&app);

    Ok(detail)
}

/// Transitions an article into `read` — the only path there, always a
/// manual user action (no automatic completion on scroll progress).
#[tauri::command]
pub async fn mark_as_read(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let pool = state.pool.clone();
    let id_for_transition = id.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok::<_, AppError>(queries::transition_to_read(&conn, &id_for_transition)?)
    })
    .await??;

    events::emit_articles_changed(&app);
    Ok(())
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

/// Deletes an article and its files (content directory + hero thumbnail,
/// if any). Deleting an id that no longer exists is treated as success —
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
        let _ = tokio::fs::remove_dir_all(state.data_dir.join("content").join(&id)).await;
        if let Some(hero) = &files.hero_image_path {
            let _ = tokio::fs::remove_file(state.data_dir.join(hero)).await;
        }
        events::emit_articles_changed(&app);
    }

    Ok(())
}

/// Deletes every article and its files in one shot — the settings
/// "delete all articles" action. Sources themselves are kept (only their
/// `article_count` is reset); this is read-later cleanup, not source
/// removal. Unlike `delete_article`, this doesn't collect a per-article
/// file list first: with everything going away, it's cheaper and just as
/// correct to wipe `content/` and `media/` wholesale.
#[tauri::command]
pub async fn delete_all_articles(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok::<_, AppError>(queries::delete_all_articles(&conn)?)
    })
    .await??;

    let _ = tokio::fs::remove_dir_all(state.data_dir.join("content")).await;
    let _ = tokio::fs::remove_dir_all(state.data_dir.join("media")).await;
    events::emit_articles_changed(&app);

    Ok(())
}

/// Re-runs the capture pipeline for an existing article against its
/// already-stored link, refreshing the readable view/content images
/// locally.
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

    let output =
        capture::capture_local(&state.http_client, &state.data_dir, &id, &existing.link).await?;

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

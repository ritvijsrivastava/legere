use tauri::{AppHandle, State};

use crate::archive_client;
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

/// Transitions an article into `reading` — called when the reader opens
/// it, whether it was previously `unread` or `read` (reopening a finished
/// article resumes it). Returns the refreshed detail immediately; if the
/// full archive is `ready` server-side but not cached locally, a download
/// is kicked off in the background (`archive_reconciler::spawn_download_if_ready`)
/// rather than awaited here, so opening an article is never blocked on a
/// slow or unreachable archive server.
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
        queries::get_article(&conn, &id_for_transition)?.ok_or_else(|| AppError::not_found("article"))
    })
    .await??;

    if detail.archive_status == "ready" && !detail.archive_available_locally {
        crate::archive_reconciler::spawn_download_if_ready(app.clone(), id.clone());
    }
    events::emit_articles_changed(&app);

    Ok(detail)
}

/// Transitions an article into `read` — the only path there, always a
/// manual user action (no automatic completion on scroll progress). If a
/// `server`-sourced article's full archive is cached locally, it's
/// evicted immediately (bytes removed, `ZimCache` entry dropped) — the
/// server retains its own copy indefinitely; nothing here ever touches it.
/// `local_legacy` archives are never evicted (see
/// `queries::evict_local_archive`'s own docs).
#[tauri::command]
pub async fn mark_as_read(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let pool = state.pool.clone();
    let id_for_transition = id.clone();
    let evicted_path = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        queries::transition_to_read(&conn, &id_for_transition)?;
        Ok::<_, AppError>(queries::evict_local_archive(&conn, &id_for_transition)?)
    })
    .await??;

    if let Some(zim_path) = evicted_path {
        let _ = tokio::fs::remove_file(state.data_dir.join(&zim_path)).await;
        state.zim_cache.evict(&id);
    }

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
        if let Some(zim_path) = &files.zim_path {
            let _ = tokio::fs::remove_file(state.data_dir.join(zim_path)).await;
        }
        if let Some(content_zim_path) = &files.content_zim_path {
            let _ = tokio::fs::remove_file(state.data_dir.join(content_zim_path)).await;
        }
        if let Some(hero) = &files.hero_image_path {
            let _ = tokio::fs::remove_file(state.data_dir.join(hero)).await;
        }
        state.zim_cache.evict(&id);
        events::emit_articles_changed(&app);
    }

    Ok(())
}

/// Re-runs the capture pipeline for an existing article against its
/// already-stored link: refreshes the readable view/content zim locally,
/// then re-submits a fresh full-page server capture job — even for a
/// `local_legacy` article (predating server-side archiving entirely),
/// which this promotes to `server`/`pending` going forward, matching a
/// brand-new capture's lifecycle.
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

    let output = capture::capture_local(&state.http_client, &state.data_dir, &id, &existing.link)
        .await?;
    let final_url = output.final_url.clone();

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

    // The content zim (and, if one was cached, the full archive) at this
    // article's path was just overwritten/reset — a cached reader from
    // before the recapture must not keep serving stale content.
    state.zim_cache.evict(&id);
    events::emit_articles_changed(&app);

    archive_client::spawn_submission(
        state.pool.clone(),
        state.server_http_client.clone(),
        id.clone(),
        final_url,
    );

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

use tauri::{AppHandle, State};

use crate::db::queries;
use crate::error::AppError;
use crate::events::{self, SyncError, SyncFinished};
use crate::models::{AddSourceAutoResult, Source, SyncResult};
use crate::sources::rss;
use crate::state::AppState;
use crate::sync::sync_all_sources;
use crate::urlx::normalize_source_url;

#[tauri::command]
pub async fn list_sources(state: State<'_, AppState>) -> Result<Vec<Source>, AppError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok(queries::list_sources(&conn)?)
    })
    .await?
}

/// Registers a recurring RSS source and triggers its first sync immediately
/// (rather than leaving it empty until the next autosync interval, up to 15
/// minutes away). Direct-link "sources" are a distinct one-shot capture,
/// not a recurring source — see `commands::articles::add_direct_link_article`.
#[tauri::command]
pub async fn add_source(
    app: AppHandle,
    state: State<'_, AppState>,
    source_type: String,
    value: String,
) -> Result<Source, AppError> {
    match source_type.as_str() {
        "rss" => insert_rss_source_and_sync(&app, &state, normalize_source_url(&value)).await,
        other => Err(AppError::Internal(format!("unknown source type: {other}"))),
    }
}

/// Shared by `add_source("rss", ...)` and `add_source_auto`'s feed branch:
/// registers the recurring source, runs its first sync inline (best-effort
/// — a failure here just leaves the source in its error state rather than
/// failing the add), and returns the row as stored.
async fn insert_rss_source_and_sync(
    app: &AppHandle,
    state: &State<'_, AppState>,
    feed_url: String,
) -> Result<Source, AppError> {
    let pool = state.pool.clone();
    let name = feed_url.clone();
    let source = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok::<_, AppError>(queries::insert_rss_source(&conn, &name, &feed_url)?)
    })
    .await??;

    if let Err(err) = sync_source_by_id(app, state, &source.id).await {
        tracing::warn!(source_id = %source.id, %err, "initial sync after add_source failed");
    }

    let pool = state.pool.clone();
    let id = source.id.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        queries::get_source(&conn, &id)?.ok_or_else(|| AppError::not_found("source"))
    })
    .await?
}

/// Single entry point for the "Add a source" dialog: the user pastes one
/// URL with no up-front RSS-vs-article choice, and this sniffs which it is
/// by fetching it once and trying to parse it as a feed. A parse failure
/// isn't distinguished from "is a webpage, not a feed" — both fall back
/// to the direct-link capture path, which is exactly the behavior a plain
/// article URL needs anyway.
#[tauri::command]
pub async fn add_source_auto(
    app: AppHandle,
    state: State<'_, AppState>,
    value: String,
) -> Result<AddSourceAutoResult, AppError> {
    let value = normalize_source_url(&value);
    let bytes = state
        .http_client
        .get(&value)
        .send()
        .await
        .map_err(|e| AppError::Network(e.to_string()))?
        .bytes()
        .await
        .map_err(|e| AppError::Network(e.to_string()))?;

    if feed_rs::parser::parse(&bytes[..]).is_ok() {
        let source = insert_rss_source_and_sync(&app, &state, value).await?;
        Ok(AddSourceAutoResult::Rss(source))
    } else {
        let article = crate::sources::direct_link::capture_direct_link(&state, &value)
            .await
            .map_err(AppError::from)?;
        events::emit_articles_changed(&app);
        Ok(AddSourceAutoResult::Direct(article))
    }
}

#[tauri::command]
pub async fn toggle_source_pause(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<Source, AppError> {
    let pool = state.pool.clone();
    let result = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok::<_, AppError>(queries::toggle_source_pause(&conn, &id)?)
    })
    .await?;
    events::emit_source_changed(&app);
    result
}

#[tauri::command]
pub async fn remove_source(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let pool = state.pool.clone();
    let result = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok::<_, AppError>(queries::remove_source(&conn, &id)?)
    })
    .await?;
    events::emit_source_changed(&app);
    result
}

/// Syncs one source by id, marking it synced (or errored) afterward. Shared
/// by the `sync_source` command and `add_source`'s immediate first sync.
async fn sync_source_by_id(app: &AppHandle, state: &AppState, id: &str) -> Result<u32, AppError> {
    let source = {
        let pool = state.pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            queries::get_source(&conn, &id)?.ok_or_else(|| AppError::not_found("source"))
        })
        .await??
    };

    events::emit_sync_started(app, &source.id);

    let result = rss::sync_rss_source(state, &source).await;

    let pool = state.pool.clone();
    let id = id.to_string();
    let sync_ok = result.is_ok();
    let error_message = result.as_ref().err().map(|e| e.to_string());
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        if sync_ok {
            Ok::<_, AppError>(queries::mark_source_synced(&conn, &id)?)
        } else {
            Ok(queries::mark_source_error(
                &conn,
                &id,
                error_message.as_deref().unwrap_or("sync failed"),
            )?)
        }
    })
    .await??;

    let new_article_count = result.as_ref().copied().unwrap_or(0);
    events::emit_sync_finished(
        app,
        &SyncFinished {
            new_article_count,
            errors: result
                .as_ref()
                .err()
                .map(|e| {
                    vec![SyncError {
                        source_id: source.id.clone(),
                        message: e.to_string(),
                    }]
                })
                .unwrap_or_default(),
        },
    );
    events::emit_source_changed(app);
    if new_article_count > 0 {
        events::emit_articles_changed(app);
    }

    result.map_err(AppError::from)
}

#[tauri::command]
pub async fn sync_source(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<SyncResult, AppError> {
    let new_article_count = sync_source_by_id(&app, &state, &id).await?;
    Ok(SyncResult { new_article_count })
}

#[tauri::command]
pub async fn sync_all(app: AppHandle, state: State<'_, AppState>) -> Result<SyncResult, AppError> {
    let new_article_count = sync_all_sources(&app, &state).await;
    Ok(SyncResult { new_article_count })
}

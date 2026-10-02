//! Settings' "Export articles" / "Import articles" — Legere's own CSV
//! shape (`title,url,category,tags,created,favourite`), independent of
//! `commands::import` (Raindrop.io migration only). See
//! `sources::article_csv` for the shared parsing/capture logic these
//! commands are thin wrappers around.

#[cfg(not(target_os = "android"))]
use std::sync::Arc;
#[cfg(not(target_os = "android"))]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(not(target_os = "android"))]
use tauri::Manager;
use tauri::{AppHandle, State};

use crate::db::queries;
use crate::error::AppError;
#[cfg(not(target_os = "android"))]
use crate::events::{self, ImportFailure, ImportFinished};
use crate::export_paths::write_export_csv;
use crate::models::ExportResult;
#[cfg(target_os = "android")]
use crate::sources::article_csv::{self, ArticleImportPreview};
#[cfg(not(target_os = "android"))]
use crate::sources::article_csv::{self, ArticleImportPreview, ImportEvent};
use crate::state::AppState;

/// Writes every article currently in the library to a CSV file under
/// `{data_dir}/exports/` and returns its path/filename for the frontend
/// to show/reveal/re-share. Synchronous — even a large library is a fast
/// single read + write, unlike an import (which fetches every link over
/// the network), so this needs no job/progress machinery.
#[tauri::command]
pub async fn export_articles_csv(state: State<'_, AppState>) -> Result<ExportResult, AppError> {
    let pool = state.pool.clone();
    let rows = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok::<_, AppError>(queries::list_articles_for_export(&conn)?)
    })
    .await??;

    let row_count = rows.len();
    let csv = article_csv::build_csv(&rows)?;
    write_export_csv(&state.data_dir, "articles", csv, row_count).await
}

/// Parses the selected CSV and returns the category-level summary without
/// capturing any links, mirroring `preview_raindrop_csv` — see its doc
/// comment.
#[tauri::command]
pub async fn preview_articles_csv(
    state: State<'_, AppState>,
    path: String,
) -> Result<ArticleImportPreview, AppError> {
    let csv_bytes = crate::csv_source::read_csv_bytes(&state.data_dir, &path).await?;
    article_csv::preview_csv(&state, &csv_bytes)
}

/// Starts a background import of Legere's own article CSV shape and
/// returns as soon as it's running — progress is tracked via the
/// `article_import:*` events, not this call's return value. Rejects a
/// second call while one is already in flight, and a file that isn't
/// even parseable, exactly like `import_raindrop_csv` (see its own doc
/// comment); kept as a fully separate guard/event stream from that
/// importer so the two can never be confused with each other on the
/// frontend even if both existed as live progress bars at once.
#[tauri::command]
pub async fn import_articles_csv(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), AppError> {
    {
        let guard = state.article_import_cancel.lock().await;
        if guard.is_some() {
            return Err(AppError::Internal(
                "an import is already running".to_string(),
            ));
        }
    }

    let resolved_path = crate::csv_source::resolve_csv_path(&state.data_dir, &path).await?;
    let csv_bytes = tokio::fs::read(&resolved_path).await?;
    article_csv::validate_csv(&csv_bytes)?;

    let pool = state.pool.clone();
    let concurrency = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok::<_, AppError>(
            queries::get_settings(&conn)?
                .import_concurrency
                .clamp(5, 10) as usize,
        )
    })
    .await??;

    // Android: handed off to a `WorkManager` job instead \u2014 see
    // `import_raindrop_csv`'s identical comment on this exact branch for
    // why `AppState::article_import_cancel` is deliberately left alone.
    #[cfg(target_os = "android")]
    {
        let _ = (&app, &csv_bytes);
        let resolved_path = resolved_path.to_string_lossy();
        crate::import_intent::enqueue("article_csv", &resolved_path, concurrency)
            .map_err(AppError::Internal)?;
        return Ok(());
    }

    #[cfg(not(target_os = "android"))]
    let cancel = Arc::new(AtomicBool::new(false));
    #[cfg(not(target_os = "android"))]
    {
        *state.article_import_cancel.lock().await = Some(cancel.clone());
    }

    #[cfg(not(target_os = "android"))]
    tauri::async_runtime::spawn(async move {
        let app_state = app.state::<AppState>();
        let result =
            article_csv::run_import(
                &app_state,
                csv_bytes,
                concurrency,
                cancel,
                |event| match event {
                    ImportEvent::Started { total } => {
                        events::emit_article_import_started(&app, total)
                    }
                    ImportEvent::Progress(progress) => {
                        events::emit_article_import_progress(&app, &progress)
                    }
                    ImportEvent::LibraryChanged => events::emit_articles_changed(&app),
                    ImportEvent::Finished(finished) => {
                        events::emit_article_import_finished(&app, &finished)
                    }
                },
            )
            .await;

        if let Err(err) = result {
            tracing::warn!(%err, "article import failed after having already started");
            events::emit_article_import_finished(
                &app,
                &ImportFinished {
                    total: 0,
                    imported: 0,
                    skipped_duplicate: 0,
                    failed: vec![ImportFailure {
                        url: String::new(),
                        title: String::new(),
                        error: err.to_string(),
                    }],
                    cancelled: false,
                },
            );
        }

        *app_state.article_import_cancel.lock().await = None;
    });

    #[cfg(not(target_os = "android"))]
    Ok(())
}

/// Cancels the in-flight article import, if any. Returns `true` if there
/// was one to cancel (on Android, always `true` — see
/// `cancel_raindrop_import`'s identical comment) — see
/// `cancel_raindrop_import` for exactly what cancelling does and doesn't
/// stop.
#[tauri::command]
pub async fn cancel_articles_import(state: State<'_, AppState>) -> Result<bool, AppError> {
    #[cfg(target_os = "android")]
    {
        let _ = &state;
        crate::import_intent::cancel().map_err(AppError::Internal)?;
        return Ok(true);
    }

    #[cfg(not(target_os = "android"))]
    let guard = state.article_import_cancel.lock().await;
    #[cfg(not(target_os = "android"))]
    match guard.as_ref() {
        Some(cancel) => {
            cancel.store(true, Ordering::Relaxed);
            Ok(true)
        }
        None => Ok(false),
    }
}

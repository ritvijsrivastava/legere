use std::collections::HashSet;

use tauri::{AppHandle, Manager, State};

use crate::capture_jobs::CaptureJob;
use crate::db::queries;
use crate::error::AppError;
use crate::events::{self, CaptureSucceeded, SyncError, SyncFinished};
use crate::export_paths::write_export_csv;
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
///
/// Runs entirely inside `spawn_capture_job`'s background task (see
/// `add_source_background`) rather than as a command itself — a slow
/// site's fetch, and an image-heavy direct-link capture's own localize
/// step, both used to block the dialog until this returned.
async fn run_add_source_auto(
    app: &AppHandle,
    state: &State<'_, AppState>,
    value: &str,
) -> Result<CaptureSucceeded, AppError> {
    let bytes = state
        .http_client
        .get(value)
        .send()
        .await
        .map_err(|e| AppError::Network(e.to_string()))?
        .bytes()
        .await
        .map_err(|e| AppError::Network(e.to_string()))?;

    if feed_rs::parser::parse(&bytes[..]).is_ok() {
        let source = insert_rss_source_and_sync(app, state, value.to_string()).await?;
        Ok(CaptureSucceeded {
            kind: "rss".to_string(),
            title: source.name,
        })
    } else {
        let article = crate::sources::direct_link::capture_direct_link(state, value)
            .await
            .map_err(AppError::from)?;
        events::emit_articles_changed(app);
        Ok(CaptureSucceeded {
            kind: "direct".to_string(),
            title: article.title,
        })
    }
}

/// Runs `run_add_source_auto(id, url)` to completion and reports the
/// outcome to `AppState::capture_jobs`: removed from the list on success
/// (plus a one-off `capture:succeeded` for the confirmation toast), left
/// in `Failed` state on error so the activity dock can offer a retry.
/// Always finishes with `capture:changed` so the dock/panel refetch
/// either way. The spawned task's own handle is attached back onto the
/// job immediately (not inside the task itself, which can't reach its
/// own handle) so `cancel_capture_job` has something to abort.
fn spawn_capture_job(app: AppHandle, id: String, url: String) {
    let handle = {
        let app = app.clone();
        let id = id.clone();
        let url = url.clone();
        tauri::async_runtime::spawn(async move {
            let state = app.state::<AppState>();
            match run_add_source_auto(&app, &state, &url).await {
                Ok(succeeded) => {
                    state.capture_jobs.succeed(&id);
                    events::emit_capture_succeeded(&app, &succeeded);
                }
                Err(err) => {
                    tracing::warn!(url = %url, %err, "background capture failed");
                    state.capture_jobs.fail(&id, err.to_string());
                }
            }
            events::emit_capture_changed(&app);
        })
    };
    app.state::<AppState>()
        .capture_jobs
        .attach_handle(&id, handle);
}

/// Queues `value` for background capture and returns immediately — see
/// `run_add_source_auto`/`spawn_capture_job`. The frontend closes the
/// "Add a source" dialog as soon as this resolves and tracks the rest via
/// `capture:*` events and `list_capture_jobs`.
#[tauri::command]
pub async fn add_source_background(
    app: AppHandle,
    state: State<'_, AppState>,
    value: String,
) -> Result<CaptureJob, AppError> {
    let value = normalize_source_url(&value);
    let job = state.capture_jobs.enqueue(value.clone());
    events::emit_capture_changed(&app);
    spawn_capture_job(app, job.id.clone(), value);
    Ok(job)
}

/// Every in-flight or failed background capture — a finished/succeeded
/// one is never returned, see `capture_jobs`'s module docs.
#[tauri::command]
pub async fn list_capture_jobs(state: State<'_, AppState>) -> Result<Vec<CaptureJob>, AppError> {
    Ok(state.capture_jobs.list())
}

/// Re-runs a failed job's URL as a fresh background attempt. Errors if
/// `id` isn't a known, non-running job.
#[tauri::command]
pub async fn retry_capture_job(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let Some(url) = state.capture_jobs.retry(&id) else {
        return Err(AppError::not_found("capture job"));
    };
    events::emit_capture_changed(&app);
    spawn_capture_job(app, id, url);
    Ok(())
}

/// Drops an acknowledged failure from the activity dock/panel without
/// retrying it.
#[tauri::command]
pub async fn dismiss_capture_job(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    state.capture_jobs.dismiss(&id);
    events::emit_capture_changed(&app);
    Ok(())
}

/// Aborts a still-running capture outright. Safe at any point in the
/// pipeline — see `capture_jobs::CaptureJobs::cancel`'s doc comment for
/// why nothing partial is left behind. Errors if `id` isn't a known,
/// running job.
#[tauri::command]
pub async fn cancel_capture_job(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    if !state.capture_jobs.cancel(&id) {
        return Err(AppError::not_found("capture job"));
    }
    events::emit_capture_changed(&app);
    Ok(())
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

/// Writes every current source to a CSV file under `{data_dir}/exports/`
/// — see `sources_csv` for the column shape. Synchronous, like
/// `export_articles_csv`: a plain DB read + file write, no network
/// involved.
#[tauri::command]
pub async fn export_sources_csv(state: State<'_, AppState>) -> Result<ExportResult, AppError> {
    let pool = state.pool.clone();
    let sources = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok::<_, AppError>(queries::list_sources(&conn)?)
    })
    .await??;

    let row_count = sources.len();
    let csv = sources_csv::build_csv(&sources)?;
    write_export_csv(&state.data_dir, "sources", csv, row_count).await
}

/// Registers a recurring RSS source and runs its first sync in the
/// background, exactly like `spawn_capture_job`/`add_source_background`
/// for a feed the sniffing step already recognized as one — except
/// `import_sources_csv` already knows every row is a feed URL (it came
/// from a previous `export_sources_csv`, or a hand-written CSV in the
/// same shape), so this skips straight to `insert_rss_source_and_sync`
/// instead of re-sniffing via `run_add_source_auto`.
fn spawn_source_import_job(app: AppHandle, id: String, feed_url: String) {
    let handle = {
        let app = app.clone();
        let id = id.clone();
        let feed_url = feed_url.clone();
        tauri::async_runtime::spawn(async move {
            let state = app.state::<AppState>();
            match insert_rss_source_and_sync(&app, &state, feed_url.clone()).await {
                Ok(source) => {
                    state.capture_jobs.succeed(&id);
                    events::emit_capture_succeeded(
                        &app,
                        &CaptureSucceeded {
                            kind: "rss".to_string(),
                            title: source.name,
                        },
                    );
                }
                Err(err) => {
                    tracing::warn!(feed_url = %feed_url, %err, "background source import failed");
                    state.capture_jobs.fail(&id, err.to_string());
                }
            }
            events::emit_capture_changed(&app);
        })
    };
    app.state::<AppState>()
        .capture_jobs
        .attach_handle(&id, handle);
}

/// Summary returned once every new row in the CSV has been *queued* for
/// background capture (not once every source has finished syncing — that
/// still runs through the same activity-dock/`capture:*`-event machinery
/// as `add_source_background`, deliberately reused rather than
/// duplicated with a second progress UI).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SourcesImportSummary {
    /// Distinct new feed URLs queued for background capture.
    pub queued: u32,
    /// Rows skipped outright because that feed URL is already a source
    /// (including duplicates within the CSV itself).
    pub skipped_duplicate: u32,
}

/// Reads `path` (a CSV in this app's own sources-export shape — see
/// `sources_csv`) and queues every feed URL not already present as a
/// background add, deduped against both the current source list and
/// other rows in the same file. Returns as soon as every new row is
/// queued; track the rest via `list_capture_jobs`/`capture:*` events,
/// exactly like `add_source_background`.
#[tauri::command]
pub async fn import_sources_csv(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<SourcesImportSummary, AppError> {
    let csv_bytes = tokio::fs::read(&path).await?;
    sources_csv::validate_csv(&csv_bytes)?;
    let feed_urls = sources_csv::parse_feed_urls(&csv_bytes)?;

    let pool = state.pool.clone();
    let mut seen: HashSet<String> = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok::<_, AppError>(
            queries::list_sources(&conn)?
                .into_iter()
                .filter_map(|s| s.feed_url)
                .collect(),
        )
    })
    .await??;

    let mut queued = 0u32;
    let mut skipped_duplicate = 0u32;
    for raw_feed_url in feed_urls {
        let feed_url = normalize_source_url(&raw_feed_url);
        if !seen.insert(feed_url.clone()) {
            skipped_duplicate += 1;
            continue;
        }
        let job = state.capture_jobs.enqueue(feed_url.clone());
        events::emit_capture_changed(&app);
        spawn_source_import_job(app.clone(), job.id.clone(), feed_url);
        queued += 1;
    }

    Ok(SourcesImportSummary {
        queued,
        skipped_duplicate,
    })
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

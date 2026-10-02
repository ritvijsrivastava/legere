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
#[cfg(not(target_os = "android"))]
use crate::sources::raindrop_import::{self, ImportEvent, ImportPreview};
#[cfg(target_os = "android")]
use crate::sources::raindrop_import::{self, ImportPreview};
use crate::state::AppState;

/// Reads `path` (a Raindrop.io CSV export chosen via the frontend's file
/// picker) and starts importing it in the background, returning as soon
/// as the import is running rather than waiting for it to finish — a
/// multi-thousand-row export can take a long time, and the frontend
/// tracks progress via the `import:*` events emitted along the way
/// instead of this command's return value.
///
/// Rejects a second call while one import is already in flight (checked
/// via `AppState::import_cancel`, which also doubles as that guard — see
/// its own doc comment) and rejects a file that isn't even a parseable
/// CSV up front, before anything is reported as started.
#[tauri::command]
pub async fn import_raindrop_csv(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), AppError> {
    {
        let guard = state.import_cancel.lock().await;
        if guard.is_some() {
            return Err(AppError::Internal(
                "an import is already running".to_string(),
            ));
        }
    }

    let resolved_path = crate::csv_source::resolve_csv_path(&state.data_dir, &path).await?;
    let csv_bytes = tokio::fs::read(&resolved_path).await?;
    raindrop_import::validate_csv(&csv_bytes)?;

    // `Settings::import_concurrency` is already clamped to 5–10 by
    // `queries::get_settings`/`update_settings`, but clamped again here
    // rather than trusted, same defensive posture as those two.
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

    // Android: handed off to a `WorkManager` job instead of an in-process
    // task, so it survives this process dying while backgrounded \u2014 see
    // `import_intent`'s module docs. `AppState::import_cancel` is
    // deliberately left untouched here: the job's own `WorkManager`
    // unique-work name is the single-import-at-a-time guard on this
    // platform, since it (unlike this in-memory `AtomicBool`) survives
    // exactly the process restarts this path exists to survive.
    #[cfg(target_os = "android")]
    {
        let _ = (&app, &csv_bytes);
        let resolved_path = resolved_path.to_string_lossy();
        crate::import_intent::enqueue("raindrop", &resolved_path, concurrency)
            .map_err(AppError::Internal)?;
        return Ok(());
    }

    #[cfg(not(target_os = "android"))]
    let cancel = Arc::new(AtomicBool::new(false));
    #[cfg(not(target_os = "android"))]
    {
        *state.import_cancel.lock().await = Some(cancel.clone());
    }

    #[cfg(not(target_os = "android"))]
    tauri::async_runtime::spawn(async move {
        let app_state = app.state::<AppState>();
        let result =
            raindrop_import::run_import(&app_state, csv_bytes, concurrency, cancel, |event| {
                match event {
                    ImportEvent::Started { total } => events::emit_import_started(&app, total),
                    ImportEvent::Progress(progress) => {
                        events::emit_import_progress(&app, &progress)
                    }
                    ImportEvent::LibraryChanged => events::emit_articles_changed(&app),
                    ImportEvent::Finished(finished) => {
                        events::emit_import_finished(&app, &finished)
                    }
                }
            })
            .await;

        // `validate_csv` above already rejects a malformed file before
        // this task is even spawned, so reaching an `Err` here would mean
        // either the file changed on disk between the two reads, or a
        // category/database error — reported the same way any other
        // row-level failure would be, rather than silently dropped.
        if let Err(err) = result {
            tracing::warn!(%err, "raindrop import failed after having already started");
            events::emit_import_finished(
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

        *app_state.import_cancel.lock().await = None;
    });

    #[cfg(not(target_os = "android"))]
    Ok(())
}

/// Parses the selected CSV and returns the folder-level summary without
/// capturing any links, so the frontend can show "N bookmarks, M already
/// saved" before starting an import — folders themselves are resolved to
/// categories automatically inside `run_import`, not chosen here.
#[tauri::command]
pub async fn preview_raindrop_csv(
    state: State<'_, AppState>,
    path: String,
) -> Result<ImportPreview, AppError> {
    let csv_bytes = crate::csv_source::read_csv_bytes(&state.data_dir, &path).await?;
    raindrop_import::preview_csv(&state, &csv_bytes)
}

/// Cancels the in-flight import, if any. Returns `true` if there was one
/// to cancel (on Android, always `true` \u2014 see `import_intent::cancel`'s
/// doc comment for why that platform can't cheaply know in advance).
/// See `raindrop_import::run_import`'s doc comment for exactly what
/// cancelling does and doesn't stop (in-flight fetches are allowed to
/// finish; only new ones are prevented).
#[tauri::command]
pub async fn cancel_raindrop_import(state: State<'_, AppState>) -> Result<bool, AppError> {
    #[cfg(target_os = "android")]
    {
        let _ = &state;
        crate::import_intent::cancel().map_err(AppError::Internal)?;
        return Ok(true);
    }

    #[cfg(not(target_os = "android"))]
    let guard = state.import_cancel.lock().await;
    #[cfg(not(target_os = "android"))]
    match guard.as_ref() {
        Some(cancel) => {
            cancel.store(true, Ordering::Relaxed);
            Ok(true)
        }
        None => Ok(false),
    }
}

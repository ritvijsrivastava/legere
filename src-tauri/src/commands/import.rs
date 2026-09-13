use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Manager, State};

use crate::error::AppError;
use crate::events::{self, ImportFailure, ImportFinished};
use crate::sources::raindrop_import::{self, ImportEvent};
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
pub async fn import_raindrop_csv(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<(), AppError> {
    {
        let guard = state.import_cancel.lock().await;
        if guard.is_some() {
            return Err(AppError::Internal("an import is already running".to_string()));
        }
    }

    let csv_bytes = tokio::fs::read(&path).await?;
    raindrop_import::validate_csv(&csv_bytes)?;

    let cancel = Arc::new(AtomicBool::new(false));
    *state.import_cancel.lock().await = Some(cancel.clone());

    tauri::async_runtime::spawn(async move {
        let app_state = app.state::<AppState>();
        let result = raindrop_import::run_import(&app_state, csv_bytes, cancel, |event| match event {
            ImportEvent::Started { total } => events::emit_import_started(&app, total),
            ImportEvent::Progress(progress) => events::emit_import_progress(&app, &progress),
            ImportEvent::LibraryChanged => events::emit_articles_changed(&app),
            ImportEvent::Finished(finished) => events::emit_import_finished(&app, &finished),
        })
        .await;

        // `validate_csv` above already rejects a malformed file before
        // this task is even spawned, so reaching an `Err` here would mean
        // the file changed on disk between the two reads — reported the
        // same way any other row-level failure would be, rather than
        // silently dropped.
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
                        error: format!("invalid CSV file: {err}"),
                    }],
                    cancelled: false,
                },
            );
        }

        *app_state.import_cancel.lock().await = None;
    });

    Ok(())
}

/// Cancels the in-flight import, if any. Returns `true` if there was one
/// to cancel. See `raindrop_import::run_import`'s doc comment for exactly
/// what cancelling does and doesn't stop (in-flight fetches are allowed
/// to finish; only new ones are prevented).
#[tauri::command]
pub async fn cancel_raindrop_import(state: State<'_, AppState>) -> Result<bool, AppError> {
    let guard = state.import_cancel.lock().await;
    match guard.as_ref() {
        Some(cancel) => {
            cancel.store(true, Ordering::Relaxed);
            Ok(true)
        }
        None => Ok(false),
    }
}

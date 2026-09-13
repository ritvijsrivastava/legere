//! Real, user-managed categories ("folders") — see `db::schema`'s `V9`
//! migration and `models::Category`'s doc comment for what distinguishes
//! this from the old `source_name`-grouped sidebar list
//! (`commands::articles::list_categories`, superseded but not yet
//! removed — see that function's own doc comment).

use tauri::{AppHandle, State};

use crate::db::queries;
use crate::error::AppError;
use crate::events;
use crate::models::Category;
use crate::state::AppState;

/// Named `get_categories` rather than `list_categories` — that name is
/// already taken by `commands::articles::list_categories` (the
/// `source_name`-grouped sidebar list, still in use until the library/
/// sidebar cut over to real categories), and Tauri commands are
/// dispatched by function name, not module path, so the two can't
/// collide.
#[tauri::command]
pub async fn count_uncategorized(state: State<'_, AppState>) -> Result<i64, AppError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok(queries::count_uncategorized(&conn)?)
    })
    .await?
}

#[tauri::command]
pub async fn get_categories(state: State<'_, AppState>) -> Result<Vec<Category>, AppError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok(queries::fetch_categories(&conn)?)
    })
    .await?
}

/// A name that case-insensitively collides with an existing category is
/// rejected with a clean message rather than the raw SQLite constraint
/// error — matched on `SqliteFailure`'s `ConstraintViolation` code rather
/// than a pre-check query, so a race between two calls still can't leave
/// a confusing raw DB error surfaced to the user.
fn is_unique_violation(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::ConstraintViolation,
                ..
            },
            _
        )
    )
}

fn map_category_write_error(name: &str, err: rusqlite::Error) -> AppError {
    if is_unique_violation(&err) {
        AppError::Internal(format!("a category named \"{name}\" already exists"))
    } else {
        AppError::from(err)
    }
}

#[tauri::command]
pub async fn create_category(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<Category, AppError> {
    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        return Err(AppError::Internal(
            "category name can't be empty".to_string(),
        ));
    }

    let pool = state.pool.clone();
    let name_for_query = trimmed.clone();
    let result = tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(AppError::from)?;
        queries::create_category(&conn, &name_for_query)
            .map_err(|e| map_category_write_error(&name_for_query, e))
    })
    .await??;

    events::emit_category_changed(&app);
    Ok(result)
}

#[tauri::command]
pub async fn rename_category(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> Result<Category, AppError> {
    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        return Err(AppError::Internal(
            "category name can't be empty".to_string(),
        ));
    }

    let pool = state.pool.clone();
    let name_for_query = trimmed.clone();
    let result = tokio::task::spawn_blocking(move || {
        let conn = pool.get().map_err(AppError::from)?;
        queries::rename_category(&conn, &id, &name_for_query)
            .map_err(|e| map_category_write_error(&name_for_query, e))?
            .ok_or_else(|| AppError::not_found("category"))
    })
    .await??;

    events::emit_category_changed(&app);
    Ok(result)
}

/// Deletes a category. Its articles aren't deleted — they fall back to
/// Uncategorized (`ON DELETE SET NULL`, see `db::schema`'s `V9`).
#[tauri::command]
pub async fn delete_category(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), AppError> {
    let pool = state.pool.clone();
    let deleted = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok::<_, AppError>(queries::delete_category(&conn, &id)?)
    })
    .await??;

    if deleted {
        events::emit_category_changed(&app);
        events::emit_articles_changed(&app);
    }
    Ok(())
}

/// Sets (or clears, for `category_id: None`) a single article's category.
#[tauri::command]
pub async fn set_article_category(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    category_id: Option<String>,
) -> Result<(), AppError> {
    let pool = state.pool.clone();
    let found = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        Ok::<_, AppError>(queries::set_article_category(
            &conn,
            &id,
            category_id.as_deref(),
        )?)
    })
    .await??;

    if !found {
        return Err(AppError::not_found("article"));
    }

    events::emit_articles_changed(&app);
    events::emit_category_changed(&app);
    Ok(())
}

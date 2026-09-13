use tauri::State;

use crate::error::AppError;
use crate::state::AppState;

/// Exposes `{app_local_data_dir}/legere` so the frontend can resolve
/// relative paths like `hero_image_path` into absolute paths for
/// `convertFileSrc`.
#[tauri::command]
pub fn get_data_dir(state: State<'_, AppState>) -> String {
    state.data_dir.to_string_lossy().into_owned()
}

/// Writes `contents` to `path`, overwriting any existing file. There's no
/// generic filesystem plugin wired up, so this is the one narrow escape
/// hatch for the frontend to save user-generated data (currently: failed
/// Raindrop import rows exported as CSV) to a location the user picked
/// via the save dialog.
#[tauri::command]
pub async fn write_text_file(path: String, contents: String) -> Result<(), AppError> {
    tokio::fs::write(&path, contents).await?;
    Ok(())
}

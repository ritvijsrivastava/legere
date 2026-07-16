use tauri::State;

use crate::state::AppState;

/// Exposes `{app_local_data_dir}/legere` so the frontend can resolve
/// relative paths like `hero_image_path` into absolute paths for
/// `convertFileSrc`.
#[tauri::command]
pub fn get_data_dir(state: State<'_, AppState>) -> String {
    state.data_dir.to_string_lossy().into_owned()
}

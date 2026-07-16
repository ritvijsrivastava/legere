use std::path::PathBuf;

use tauri::async_runtime::JoinHandle;
use tokio::sync::Mutex;

use crate::db::DbPool;

pub struct AppState {
    pub pool: DbPool,
    pub http_client: reqwest::Client,
    /// `{app_local_data_dir}/legere` — parent of `media/` and `archives/`.
    pub data_dir: PathBuf,
    pub autosync_handle: Mutex<Option<JoinHandle<()>>>,
}

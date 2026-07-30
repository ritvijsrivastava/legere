use std::path::PathBuf;
use std::sync::Mutex as StdMutex;
use std::time::Instant;

use tauri::async_runtime::JoinHandle;
use tokio::sync::Mutex;

use crate::db::DbPool;
use crate::zim_server::ZimCache;

pub struct AppState {
    pub pool: DbPool,
    pub http_client: reqwest::Client,
    /// `{app_local_data_dir}/legere` — parent of `media/` and `archives/`.
    pub data_dir: PathBuf,
    pub autosync_handle: Mutex<Option<JoinHandle<()>>>,
    pub zim_cache: ZimCache,
    /// Set on mobile's `RunEvent::Resumed`, throttling foreground-sync to
    /// once per interval — there's no background autosync on Android (no
    /// WorkManager integration in the MVP), so this is the only sync
    /// trigger between app launches beyond a manual refresh.
    #[cfg_attr(not(mobile), allow(dead_code))]
    pub last_foreground_sync: StdMutex<Option<Instant>>,
}

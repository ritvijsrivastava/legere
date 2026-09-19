use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Instant;

use tauri::async_runtime::JoinHandle;
use tokio::sync::Mutex;

use crate::capture_jobs::CaptureJobs;
use crate::db::DbPool;

pub struct AppState {
    pub pool: DbPool,
    pub http_client: reqwest::Client,
    /// `{app_local_data_dir}/legere` — parent of `media/` and `content/`.
    pub data_dir: PathBuf,
    pub autosync_handle: Mutex<Option<JoinHandle<()>>>,
    /// Set on mobile's `RunEvent::Resumed`, throttling foreground-sync to
    /// once per interval — there's no background autosync on Android (no
    /// WorkManager integration in the MVP), so this is the only sync
    /// trigger between app launches beyond a manual refresh.
    #[cfg_attr(not(mobile), allow(dead_code))]
    pub last_foreground_sync: StdMutex<Option<Instant>>,
    /// Cancellation flag for an in-progress `raindrop_import::run_import`
    /// run, `None` when no import is running. A command starting an
    /// import stores the flag it handed to the background task here so
    /// `cancel_raindrop_import` can flip it; the task itself clears this
    /// back to `None` when it finishes (successfully, on error, or via
    /// cancellation), which also doubles as the single-import-at-a-time
    /// guard (a start request while this is `Some` is rejected).
    pub import_cancel: Mutex<Option<Arc<AtomicBool>>>,
    /// Cancellation flag for an in-progress `exports::articles::run_import`
    /// run (Settings' "Import articles", Legere's own CSV shape) — separate
    /// from `import_cancel` (the Raindrop-migration importer) so the two
    /// can't be confused with each other, even though only one importer of
    /// either kind is ever allowed to run at a time in practice.
    pub article_import_cancel: Mutex<Option<Arc<AtomicBool>>>,
    /// In-flight/failed "add a source" background captures — see
    /// `capture_jobs`'s module docs for why this is in-memory only.
    pub capture_jobs: CaptureJobs,
    /// Update found by `check_for_update`, consumed by `install_update`.
    /// tauri-plugin-updater doesn't support mobile, so this only exists on desktop.
    #[cfg(not(target_os = "android"))]
    pub pending_update: Mutex<Option<tauri_plugin_updater::Update>>,
    /// Update found by `linux_check_for_update`, consumed by
    /// `linux_install_update` — see `commands::update_linux`. Only ever
    /// populated on Linux, but present for the whole not(android) build since
    /// it shares `AppState` with `pending_update`.
    #[cfg(not(target_os = "android"))]
    pub pending_linux_update: Mutex<Option<crate::commands::update_linux::LinuxUpdate>>,
    /// Android equivalent of `pending_update` — see `commands::update_android`.
    /// Only present when the `apk-self-update` feature is on (the
    /// GitHub-sideload build); a plain dev/local Android build has no
    /// self-update mechanism at all and needs no pending-update state.
    #[cfg(all(target_os = "android", feature = "apk-self-update"))]
    pub pending_android_update: Mutex<Option<crate::commands::update_android::AndroidUpdate>>,
}

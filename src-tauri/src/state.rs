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
    /// Plain (not SSRF-guarded) client for the self-update checks/downloads
    /// in `commands::update`/`update_android`/`update_linux`. Those hit
    /// hardcoded, first-party URLs (`api.github.com` and wherever it
    /// redirects release assets to) rather than attacker-influenced page
    /// content, so `capture::ssrf::ssrf_guarded_client_builder`'s resolver
    /// doesn't apply — and must not be reused here, since it unconditionally
    /// refuses to connect to anything that resolves into a private/
    /// link-local/CGNAT range, which plenty of legitimate networks (VPNs,
    /// corporate DNS, carrier-grade NAT, emulator host-NAT) route ordinary
    /// public traffic through before the public IP. Same reasoning
    /// `remote_sync::client::S3Client` already uses its own plain client for.
    pub update_http_client: reqwest::Client,
    /// `{app_local_data_dir}/legere` — parent of `media/` and `content/`.
    pub data_dir: PathBuf,
    pub autosync_handle: Mutex<Option<JoinHandle<()>>>,
    /// The cross-device sync hourly loop's handle, separate from
    /// `autosync_handle` (RSS sources): the two run on independent
    /// schedules and are toggled independently. See `commands::remote_sync`.
    pub remote_sync_handle: Mutex<Option<JoinHandle<()>>>,
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
    /// Cancellation flag for an in-progress `remote_sync::engine::run_sync`
    /// pass. Same dual-purpose pattern as `import_cancel`: `Some` also
    /// means "a cross-device sync is already running," which
    /// `orchestrate::run_remote_sync_once` checks to refuse a second
    /// concurrent run (from the hourly scheduler racing a manual "Sync
    /// now", or vice versa) rather than letting two passes run at once.
    /// Also flipped when the user turns sync off while one is running.
    pub remote_sync_cancel: Mutex<Option<Arc<AtomicBool>>>,
    /// In-flight/failed "add a source" background captures — see
    /// `capture_jobs`'s module docs for why this is in-memory only.
    pub capture_jobs: CaptureJobs,
    /// The cross-device sync `S3Client` built from the last-used
    /// `RemoteSyncConfig`, kept alive (and its underlying `reqwest::Client`'s
    /// connection pool/TLS sessions with it) across calls instead of
    /// rebuilding one from scratch every scheduled/manual sync pass *and*
    /// every lazy per-article image pull (`orchestrate::ensure_article_images_synced`
    /// runs on every `open_for_reading`, which is far more often than a sync
    /// pass). Keyed by the config itself (cheap to `Clone`/`PartialEq`) so a
    /// saved config change is picked up automatically on the next use — no
    /// separate invalidation path needed. See `remote_sync::orchestrate::cached_client`.
    pub remote_sync_client_cache: Mutex<
        Option<(
            crate::db::sync_config::RemoteSyncConfig,
            crate::remote_sync::client::S3Client,
        )>,
    >,
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

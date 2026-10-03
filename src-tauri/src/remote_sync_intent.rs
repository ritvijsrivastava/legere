//! JNI entrypoint backing Android's periodic background cross-device
//! sync: `WorkManager` (`RemoteSyncWorker.kt`) runs this on its own
//! schedule, independent of whether this app's own Tauri
//! runtime/`AppState` is alive in this process — mirrors
//! `share_intent.rs`'s running-app-or-standalone-fallback shape (see that
//! module's docs for the fuller rationale behind the split).
//!
//! Desktop has no equivalent: its own autosync loop
//! (`remote_sync::orchestrate::spawn_remote_sync_autosync`) just runs
//! inside the always-alive desktop process for as long as it's open.
//! Android has no such guarantee — the OS suspends/kills the process in
//! the background — so this is the mechanism that actually makes
//! cross-device sync fire on a schedule on Android at all; the same
//! loop is spawned there too (`lib.rs`'s `setup`), but only ever actually
//! ticks while the app happens to be in the foreground. Both run on the
//! same user-configured interval (`RemoteSyncConfig::sync_interval_hours`)
//! — see `getRemoteSyncIntervalHours` below for how the Android
//! `WorkManager` side picks that value up.

use std::path::PathBuf;

use jni::EnvUnowned;
use jni::errors::LogErrorAndDefault;
use jni::objects::{JClass, JObject, JString};
use serde::Serialize;
use tauri::Manager;

use crate::db;
use crate::remote_sync::engine;

#[derive(Serialize)]
struct RemoteSyncResult {
    ok: bool,
    /// `true` when sync isn't configured/enabled on this device — not a
    /// failure, just nothing to do (mirrors
    /// `orchestrate::run_remote_sync_once`'s `Ok(None)`).
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pulled: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pushed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl RemoteSyncResult {
    fn skipped() -> Self {
        Self {
            ok: true,
            skipped: Some(true),
            pulled: None,
            pushed: None,
            error: None,
        }
    }

    fn ok(pulled: usize, pushed: usize) -> Self {
        Self {
            ok: true,
            skipped: None,
            pulled: Some(pulled),
            pushed: Some(pushed),
            error: None,
        }
    }

    fn err(error: impl ToString) -> Self {
        Self {
            ok: false,
            skipped: None,
            pulled: None,
            pushed: None,
            error: Some(error.to_string()),
        }
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).expect("RemoteSyncResult always serializes")
    }
}

/// Called from `NativeSync.runRemoteSyncOnce` on a `WorkManager`
/// background thread; blocks it for the duration of one sync pass.
/// `context`/`data_dir` have the exact same meaning and provenance as
/// `share_intent.rs`'s `captureSharedUrl` — see that function's doc
/// comment. Never throws; every failure comes back as JSON so
/// `RemoteSyncWorker` can show a real (if terse) notification reason.
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_com_ritvijsrivastava_legere_NativeSync_runRemoteSyncOnce<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    context: JObject<'local>,
    data_dir: JString<'local>,
) -> jni::sys::jstring {
    let data_dir = env
        .with_env(|env| -> jni::errors::Result<String> {
            if let Err(error) = crate::mobile_tls::init_tls(env, context) {
                tracing::error!(%error, "background sync: failed to init TLS context");
            }
            data_dir.try_to_string(env)
        })
        .resolve::<LogErrorAndDefault>();

    let result = if let Some(result) = try_sync_via_running_app() {
        result
    } else if data_dir.is_empty() {
        RemoteSyncResult::err("missing data_dir")
    } else {
        run_standalone_sync(PathBuf::from(data_dir))
    };
    let json = result.to_json();

    env.with_env(|env| -> jni::errors::Result<jni::sys::jstring> {
        Ok(env.new_string(&json)?.into_raw())
    })
    .resolve::<LogErrorAndDefault>()
}

/// If this process already has a running Tauri app instance, routes
/// through its real `AppState` (`orchestrate::run_remote_sync_once`) —
/// reusing its single-sync-at-a-time guard and its own already-ticking
/// hourly loop's bookkeeping — rather than starting a redundant second,
/// standalone pass against the same bucket. `None` (meaning "fall back
/// to `run_standalone_sync`") if there's no running instance to reuse.
fn try_sync_via_running_app() -> Option<RemoteSyncResult> {
    let app = crate::GLOBAL_APP_HANDLE.get()?.clone();
    let result = tauri::async_runtime::block_on(async move {
        let state = app.state::<crate::state::AppState>();
        crate::remote_sync::orchestrate::run_remote_sync_once(&app, &state).await
    });
    Some(match result {
        Ok(Some(outcome)) => RemoteSyncResult::ok(outcome.pulled, outcome.pushed),
        Ok(None) => RemoteSyncResult::skipped(),
        Err(error) => RemoteSyncResult::err(error),
    })
}

/// Builds a throwaway DB pool + Tokio runtime and runs one sync pass
/// directly, with no `AppState`/event emission — there's nothing in this
/// process listening for `remote-sync:*` events anyway when this path
/// runs at all (it only runs when there's no live app instance). Mirrors
/// `share_intent.rs::run_capture`'s same "build everything from scratch"
/// shape, including migrating the schema first (a periodic sync could in
/// principle fire before the app has ever been opened once, however
/// unlikely in practice, since sync itself has to have been configured
/// through the app first).
fn run_standalone_sync(data_dir: PathBuf) -> RemoteSyncResult {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => return RemoteSyncResult::err(error),
    };
    runtime.block_on(async move {
        if let Err(error) = tokio::fs::create_dir_all(&data_dir).await {
            return RemoteSyncResult::err(error);
        }

        let pool = match db::build_pool(&data_dir.join("legere.db")) {
            Ok(pool) => pool,
            Err(error) => return RemoteSyncResult::err(error),
        };

        let migrated = {
            let pool = pool.clone();
            tokio::task::spawn_blocking(move || -> Result<(), String> {
                let mut conn = pool.get().map_err(|error| error.to_string())?;
                db::schema::migrate(&mut conn).map_err(|error| error.to_string())
            })
            .await
        };
        match migrated {
            Ok(Ok(())) => {}
            Ok(Err(message)) => return RemoteSyncResult::err(message),
            Err(join_error) => return RemoteSyncResult::err(join_error),
        }

        let config = {
            let pool = pool.clone();
            tokio::task::spawn_blocking(move || -> Result<_, String> {
                let conn = pool.get().map_err(|e| e.to_string())?;
                db::sync_config::get_remote_sync_config(&conn).map_err(|e| e.to_string())
            })
            .await
        };
        let config = match config {
            Ok(Ok(Some(config))) if config.enabled => config,
            Ok(Ok(_)) => return RemoteSyncResult::skipped(),
            Ok(Err(message)) => return RemoteSyncResult::err(message),
            Err(join_error) => return RemoteSyncResult::err(join_error),
        };

        let client = match engine::client_from_config(&config) {
            Ok(client) => client,
            Err(error) => return RemoteSyncResult::err(error),
        };

        // Flattened to `Result<_, String>` inside the closure rather than
        // threading `engine::SyncError` back out: this standalone path
        // has no `AppState` to distinguish a cancelled pass from a real
        // failure for (there's no cancel button to have triggered one),
        // so a plain string is all the caller needs.
        let sync_result: Result<Result<engine::SyncOutcome, String>, tokio::task::JoinError> = {
            let pool = pool.clone();
            let data_dir = data_dir.clone();
            tokio::task::spawn_blocking(move || -> Result<engine::SyncOutcome, String> {
                let conn = pool.get().map_err(|e| e.to_string())?;
                let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                tauri::async_runtime::block_on(engine::run_sync(
                    &conn,
                    &client,
                    &data_dir,
                    chrono::Utc::now(),
                    &cancel,
                    |_progress| {},
                ))
                .map_err(|e| e.to_string())
            })
            .await
        };

        match sync_result {
            Ok(Ok(outcome)) => {
                let pool = pool.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    pool.get()
                        .ok()
                        .map(|conn| db::sync_config::record_sync_success(&conn))
                })
                .await;
                RemoteSyncResult::ok(outcome.pulled, outcome.pushed)
            }
            Ok(Err(message)) => {
                let pool = pool.clone();
                let message_clone = message.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    pool.get()
                        .ok()
                        .map(|conn| db::sync_config::record_sync_failure(&conn, &message_clone))
                })
                .await;
                RemoteSyncResult::err(message)
            }
            Err(join_error) => RemoteSyncResult::err(join_error),
        }
    })
}

/// Called from `NativeSync.getRemoteSyncIntervalHours` on `MainActivity`
/// launch, to (re)schedule `RemoteSyncWorker`'s periodic `WorkManager`
/// job at the user's currently configured cadence (`RemoteSyncConfig
/// ::sync_interval_hours`) rather than a hardcoded one -- see
/// `RemoteSyncWorker.schedulePeriodic`'s doc comment for why this only
/// takes effect on the next app launch rather than instantly when the
/// setting changes. Synchronous (no `tokio` runtime needed: reading
/// config is plain `rusqlite`) and defaults to 6 on any failure
/// (missing/unreadable DB, not yet configured, ...) -- never throws.
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_com_ritvijsrivastava_legere_NativeSync_getRemoteSyncIntervalHours<
    'local,
>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    _context: JObject<'local>,
    data_dir: JString<'local>,
) -> jni::sys::jint {
    const DEFAULT_HOURS: i32 = 6;

    let data_dir = env
        .with_env(|env| -> jni::errors::Result<String> { data_dir.try_to_string(env) })
        .resolve::<LogErrorAndDefault>();
    if data_dir.is_empty() {
        return DEFAULT_HOURS;
    }

    // Called from `MainActivity.onCreate`'s background thread, but must
    // still stay cheap: no pool (r2d2's `build()` blocks up to its 30s
    // connection timeout retrying an unopenable file, e.g. on a fresh
    // install before `setup()` has created the data dir) -- a missing DB
    // just means "not configured yet", so skip straight to the default.
    let db_path = PathBuf::from(data_dir).join("legere.db");
    if !db_path.exists() {
        return DEFAULT_HOURS;
    }
    rusqlite::Connection::open(&db_path)
        .ok()
        .and_then(|conn| {
            conn.busy_timeout(std::time::Duration::from_millis(1000))
                .ok()?;
            db::sync_config::get_remote_sync_config(&conn).ok()
        })
        .flatten()
        .map(|config| config.sync_interval_hours as i32)
        .unwrap_or(DEFAULT_HOURS)
}

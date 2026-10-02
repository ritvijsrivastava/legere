//! Android only: makes a Raindrop/article CSV import survive this app's
//! own process dying (backgrounded long enough for the OS to kill it, or
//! swiped away from Recents) — something `commands::import`/
//! `commands::export`'s plain `tauri::async_runtime::spawn` can't do on
//! its own, since that task dies with the process exactly like every
//! other in-process future. Desktop has no equivalent: its process stays
//! alive for as long as the window is open (see `lib.rs`'s quit-blocking
//! `WindowEvent::CloseRequested` guard for *that* platform's own answer
//! to "don't lose an in-progress import").
//!
//! The fix is the same shape as `share_intent.rs`/`remote_sync_intent.rs`:
//! hand the actual import off to a `WorkManager` job (`ImportWorker.kt`),
//! which Android keeps alive independently of this app's own
//! Activity/webview, restarting it in a fresh process if necessary. The
//! one new wrinkle those two modules don't have: starting a `WorkManager`
//! job has to be *triggered*, and unlike a share (an OS-level intent
//! Android itself delivers to `ShareActivity`) or a periodic sync tick
//! (scheduled once from `MainActivity.onCreate`, see
//! `RemoteSyncWorker.schedulePeriodic`), an import starts from a normal
//! Tauri IPC command fired by the user tapping "Import" inside the
//! already-running app — so *Rust* has to be the one calling *into*
//! Kotlin here, the opposite direction from every other JNI bridge in
//! this codebase. `tauri::App`'s own equivalent capability
//! (`run_on_android_context`) is `pub(crate)`-sealed to the `tauri` crate
//! itself and not reachable from application code, and a full custom
//! Tauri mobile plugin (a second crate + Gradle wiring) is disproportionate
//! for one feature — so this reuses the same minimal pattern
//! `mobile_tls.rs`'s `initTls()` already established: `MainActivity`
//! hands Rust a `GlobalRef` to itself once, early in `onCreate`
//! (`cacheImportActivity` below), and Rust calls a plain (non-`external`)
//! method on that cached reference whenever it later needs to reach back
//! into Kotlin. Since an import can only ever be *started* while that
//! Activity is alive, the cached reference is guaranteed valid at the
//! one moment it's actually needed; once `MainActivity.enqueueImportWork`
//! hands the job to `WorkManager`, the job is durable from then on,
//! independent of whatever happens to the Activity or process afterward.
//!
//! Progress reporting back to a notification works differently from
//! `ShareWorker`'s single on/off state: an import can run for minutes, so
//! [`ImportWorker.kt`] polls [`Java_com_ritvijsrivastava_legere_NativeImport_pollImportProgress`]
//! on a timer while [`Java_com_ritvijsrivastava_legere_NativeImport_runImport`]
//! blocks a different thread, rather than waiting for one final result —
//! a push callback from Rust back into Java mid-import would work too, but
//! polling a plain JSON snapshot needs no new JNI object-callback plumbing
//! and is simpler to get right.
//!
//! Cancelling (the user's own "Cancel" button, `commands::import::cancel_raindrop_import`/
//! `commands::export::cancel_articles_import`) calls
//! `MainActivity.cancelImportWork`, which cancels the named `WorkManager`
//! job; `ImportWorker.onStopped()` then calls
//! [`Java_com_ritvijsrivastava_legere_NativeImport_cancelImport`] to flip
//! the same `AtomicBool` `run_import`'s own cooperative cancellation
//! already reads — the exact mechanism `raindrop_import::run_import`'s
//! module docs describe, just reached from a different direction. The
//! "only one import at a time" guard is `WorkManager`'s own unique-work
//! name (`ImportWorker`'s `ExistingWorkPolicy.KEEP`), not `AppState`'s
//! `import_cancel`/`article_import_cancel` (which this whole path leaves
//! untouched on Android) — unlike the in-memory `AtomicBool` guard those
//! fields back, the unique-work name survives exactly the process
//! restarts this module exists to survive.
//!
//! Known limitation: reopening the app mid-import (especially after a
//! process restart) doesn't resync the frontend's `importStore`/
//! `articleImportStore` to the already-in-progress job's current state —
//! `import:*`/`article_import:*` events only fire while *this* run is
//! using the running-app path below. The notification remains the
//! authoritative progress indicator in that case; the library view
//! simply shows the final result once the import finishes and
//! `articles:changed` fires (or on next manual refresh).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};

use jni::EnvUnowned;
use jni::errors::LogErrorAndDefault;
use jni::objects::{JClass, JObject, JString};
use jni::refs::Global;
use jni::signature::RuntimeMethodSignature;
use jni::vm::JavaVM;
use jni::{JValue, errors::Result as JniResult};
use serde::Serialize;
use tauri::Manager;

use crate::capture::fetch::build_client;
use crate::db;
use crate::error::AppError;
use crate::sources::{article_csv, raindrop_import};
use crate::state::AppState;

/// A `GlobalRef` to the last-created `MainActivity`, handed over by
/// `cacheImportActivity` — see this module's own doc comment for why
/// Rust needs this at all (there's no other sanctioned way for
/// application code, as opposed to a full Tauri plugin, to call back into
/// Kotlin). Re-cached (not just set-once) on every `onCreate`, so a
/// recreated Activity (rotation, process trim) doesn't leave a stale
/// reference behind — though in practice an import is only ever started
/// from a freshly visible screen, so the very latest one is always the
/// right one anyway.
static CACHED_ACTIVITY: OnceLock<StdMutex<Option<(JavaVM, Global<JObject<'static>>)>>> =
    OnceLock::new();

/// Cooperative cancellation for whichever import `NativeImport.runImport`
/// is currently running, flipped by `cancelImport` (itself called from
/// `ImportWorker.onStopped()`). Process-global rather than living on
/// `AppState` because this has to work even when there's no `AppState` in
/// this process at all (the standalone fallback below) — unlike
/// `AppState::import_cancel`, which this whole module leaves alone.
static IMPORT_CANCEL: OnceLock<StdMutex<Option<Arc<AtomicBool>>>> = OnceLock::new();

/// The latest progress snapshot for whichever import is currently
/// running, polled by `ImportWorker` to keep its notification's progress
/// bar current. `None` once read by a `Finished` state and before any
/// import has started.
static IMPORT_PROGRESS: OnceLock<StdMutex<Option<ProgressSnapshot>>> = OnceLock::new();

#[derive(Default, Serialize, Clone)]
struct ProgressSnapshot {
    total: u32,
    processed: u32,
    imported: u32,
    skipped_duplicate: u32,
    failed: u32,
    finished: bool,
    cancelled: bool,
    error: Option<String>,
}

fn set_progress(snapshot: ProgressSnapshot) {
    *IMPORT_PROGRESS
        .get_or_init(Default::default)
        .lock()
        .unwrap() = Some(snapshot);
}

/// Called once, early in `MainActivity.onCreate` (alongside `initTls()`),
/// handing Rust a `GlobalRef` to the Activity instance and its `JavaVM`
/// so `enqueue`/`cancel` below can call back into it later — see this
/// module's own doc comment for the full rationale.
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_com_ritvijsrivastava_legere_MainActivity_cacheImportActivity<'local>(
    mut env: EnvUnowned<'local>,
    this: JObject<'local>,
) {
    env.with_env(|env| -> JniResult<()> {
        let vm = env.get_java_vm()?;
        let activity = env.new_global_ref(this)?;
        *CACHED_ACTIVITY
            .get_or_init(Default::default)
            .lock()
            .unwrap() = Some((vm, activity));
        Ok(())
    })
    .resolve::<LogErrorAndDefault>();
}

/// Hands `path` (already-resolved on the Rust side, same as the
/// desktop/in-process path — see `commands::import::import_raindrop_csv`)
/// off to `MainActivity.enqueueImportWork`, which schedules the actual
/// `ImportWorker` job. `kind` is `"raindrop"` or `"article_csv"`.
///
/// Fails only if no `MainActivity` has ever run in this process (an
/// import can't be started from anywhere else, so this should never
/// actually happen in practice) or the JNI call itself errors.
pub fn enqueue(kind: &str, path: &str, concurrency: usize) -> Result<(), String> {
    with_cached_activity(|env, activity| {
        let kind_j = env.new_string(kind)?;
        let path_j = env.new_string(path)?;
        let sig = RuntimeMethodSignature::from_str("(Ljava/lang/String;Ljava/lang/String;I)V")?;
        env.call_method(
            activity,
            jni::strings::JNIString::from("enqueueImportWork"),
            sig.method_signature(),
            &[
                JValue::Object(&kind_j),
                JValue::Object(&path_j),
                JValue::Int(concurrency as i32),
            ],
        )?;
        Ok(())
    })
}

/// Cancels whichever `WorkManager` import job is running (unconditionally
/// — `WorkManager.cancelUniqueWork` is a no-op if there isn't one), via
/// `MainActivity.cancelImportWork`. `commands::import::cancel_raindrop_import`/
/// `commands::export::cancel_articles_import` call this on Android
/// instead of flipping `AppState::import_cancel` directly.
pub fn cancel() -> Result<(), String> {
    with_cached_activity(|env, activity| {
        let sig = RuntimeMethodSignature::from_str("()V")?;
        env.call_method(
            activity,
            jni::strings::JNIString::from("cancelImportWork"),
            sig.method_signature(),
            &[],
        )?;
        Ok(())
    })
}

/// Copies a SAF `content://` URI to a real path via `MainActivity.
/// copyContentUriToFile` — see `csv_source.rs`'s module docs for why this
/// exists at all. `dest` must already have an existing parent directory.
pub fn copy_content_uri_to_path(uri: &str, dest: &str) -> Result<(), String> {
    with_cached_activity(|env, activity| {
        let uri_j = env.new_string(uri)?;
        let dest_j = env.new_string(dest)?;
        let sig = RuntimeMethodSignature::from_str("(Ljava/lang/String;Ljava/lang/String;)Z")?;
        let result = env.call_method(
            activity,
            jni::strings::JNIString::from("copyContentUriToFile"),
            sig.method_signature(),
            &[JValue::Object(&uri_j), JValue::Object(&dest_j)],
        )?;
        let ok = result.z()?;
        if !ok {
            // `MainActivity.copyContentUriToFile` swallows the real
            // exception (see its own doc comment) — there's nothing more
            // specific to report here than "it didn't work".
            return Err(jni::errors::Error::JniCall(jni::errors::JniError::Unknown));
        }
        Ok(())
    })
}

fn with_cached_activity(
    f: impl FnOnce(&mut jni::Env, &JObject) -> JniResult<()>,
) -> Result<(), String> {
    let cell = CACHED_ACTIVITY
        .get()
        .ok_or("no Activity has registered itself yet")?;
    let guard = cell.lock().unwrap();
    let (vm, activity) = guard
        .as_ref()
        .ok_or("no Activity has registered itself yet")?;
    vm.attach_current_thread(|env| f(env, activity))
        .map_err(|error: jni::errors::Error| error.to_string())
}

/// `ImportWorker`'s own progress-bar notification, polled on a timer
/// while [`run_import`] below blocks a different thread. Returns a JSON
/// `ProgressSnapshot`, or `"null"` before any import has started/after
/// the last one's been read and cleared.
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_com_ritvijsrivastava_legere_NativeImport_pollImportProgress<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> jni::sys::jstring {
    let snapshot = IMPORT_PROGRESS
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .clone();
    let json = serde_json::to_string(&snapshot).unwrap_or_else(|_| "null".to_string());
    env.with_env(|env| -> JniResult<jni::sys::jstring> { Ok(env.new_string(&json)?.into_raw()) })
        .resolve::<LogErrorAndDefault>()
}

/// Flips whichever import's cancellation flag is currently active —
/// called from `ImportWorker.onStopped()`. A no-op if nothing's running.
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_com_ritvijsrivastava_legere_NativeImport_cancelImport<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
) {
    if let Some(cancel) = IMPORT_CANCEL
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .as_ref()
    {
        cancel.store(true, Ordering::Relaxed);
    }
}

#[derive(Serialize)]
struct ImportResult {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl ImportResult {
    fn ok() -> Self {
        Self {
            ok: true,
            error: None,
        }
    }

    fn err(error: impl ToString) -> Self {
        Self {
            ok: false,
            error: Some(error.to_string()),
        }
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).expect("ImportResult always serializes")
    }
}

/// `ImportWorker.doWork()`'s entrypoint — blocks the calling thread for
/// the whole import, exactly like `NativeCapture.captureSharedUrl`/
/// `NativeSync.runRemoteSyncOnce`. `kind` is `"raindrop"` or
/// `"article_csv"`; `context`/`data_dir` have the same meaning as those
/// two functions' identical parameters (only used by the standalone
/// fallback below).
///
/// Returns a JSON `{"ok":true}` or `{"ok":false,"error":"..."}` — never
/// throws. Per-row failures within a successful run aren't reported here
/// at all (same as the desktop/foreground path): they're visible via
/// `import:*`/`article_import:*` events when routed through a running
/// app, or simply absent from the final notification's count otherwise.
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_com_ritvijsrivastava_legere_NativeImport_runImport<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    context: JObject<'local>,
    data_dir: JString<'local>,
    kind: JString<'local>,
    csv_path: JString<'local>,
    concurrency: jni::sys::jint,
) -> jni::sys::jstring {
    let inputs = env
        .with_env(|env| -> JniResult<(String, String, String, String)> {
            if let Err(error) = crate::mobile_tls::init_tls(env, context) {
                tracing::error!(%error, "import: failed to init TLS context");
            }
            let data_dir = data_dir.try_to_string(env)?;
            let kind = kind.try_to_string(env)?;
            let csv_path = csv_path.try_to_string(env)?;
            Ok((data_dir, kind, csv_path, String::new()))
        })
        .resolve::<LogErrorAndDefault>();

    let (data_dir, kind, csv_path, _) = inputs;
    set_progress(ProgressSnapshot::default());

    let cancel = Arc::new(AtomicBool::new(false));
    *IMPORT_CANCEL.get_or_init(Default::default).lock().unwrap() = Some(cancel.clone());

    let result = if kind.is_empty() || csv_path.is_empty() {
        ImportResult::err("missing arguments")
    } else {
        let runtime = tauri::async_runtime::block_on(run_import(
            kind,
            csv_path,
            concurrency.max(1) as usize,
            cancel,
            PathBufOrEmpty(data_dir),
        ));
        runtime
    };

    *IMPORT_CANCEL.get_or_init(Default::default).lock().unwrap() = None;

    let json = result.to_json();
    env.with_env(|env| -> JniResult<jni::sys::jstring> { Ok(env.new_string(&json)?.into_raw()) })
        .resolve::<LogErrorAndDefault>()
}

/// Thin wrapper so the signature above reads cleanly — `data_dir` is
/// only ever used by the standalone fallback inside [`run_import`].
struct PathBufOrEmpty(String);

async fn run_import(
    kind: String,
    csv_path: String,
    concurrency: usize,
    cancel: Arc<AtomicBool>,
    data_dir: PathBufOrEmpty,
) -> ImportResult {
    let csv_bytes = match tokio::fs::read(&csv_path).await {
        Ok(bytes) => bytes,
        Err(error) => return ImportResult::err(error),
    };

    if let Some(app) = crate::GLOBAL_APP_HANDLE.get().cloned() {
        let state = app.state::<AppState>();
        let result = run_via_state(&app, &state, &kind, csv_bytes, concurrency, cancel).await;
        return result;
    }

    let data_dir = std::path::PathBuf::from(data_dir.0);
    if data_dir.as_os_str().is_empty() {
        return ImportResult::err("missing data directory");
    }
    let state = match build_standalone_state(&data_dir).await {
        Ok(state) => state,
        Err(error) => return ImportResult::err(error),
    };
    run_via_state_owned(&state, &kind, csv_bytes, concurrency, cancel).await
}

/// Routes the import through a real, already-running `AppState` — the
/// app happened to still be alive in this process — so a still-open
/// library/import dialog keeps seeing live `import:*`/`article_import:*`
/// events exactly as it would on desktop, on top of updating the
/// notification-backing [`ProgressSnapshot`] either way.
async fn run_via_state(
    app: &tauri::AppHandle,
    state: &AppState,
    kind: &str,
    csv_bytes: Vec<u8>,
    concurrency: usize,
    cancel: Arc<AtomicBool>,
) -> ImportResult {
    let app_for_events = app.clone();
    let result = match kind {
        "raindrop" => raindrop_import::run_import(state, csv_bytes, concurrency, cancel, |event| {
            apply_raindrop_event(&event, &app_for_events);
        })
        .await
        .map(|_| ()),
        "article_csv" => article_csv::run_import(state, csv_bytes, concurrency, cancel, |event| {
            apply_article_event(&event, &app_for_events);
        })
        .await
        .map(|_| ()),
        other => Err(AppError::Internal(format!("unknown import kind: {other}"))),
    };

    match result {
        Ok(()) => ImportResult::ok(),
        Err(error) => {
            let mut snapshot = IMPORT_PROGRESS
                .get_or_init(Default::default)
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_default();
            snapshot.finished = true;
            snapshot.error = Some(error.to_string());
            set_progress(snapshot);
            ImportResult::err(error)
        }
    }
}

/// Same as [`run_via_state`] but for the standalone fallback, which owns
/// its `AppState` outright rather than borrowing one from `GLOBAL_APP_HANDLE`
/// — there's no `AppHandle` to emit Tauri events to in this case, only
/// the `ProgressSnapshot` notification polling reads.
async fn run_via_state_owned(
    state: &AppState,
    kind: &str,
    csv_bytes: Vec<u8>,
    concurrency: usize,
    cancel: Arc<AtomicBool>,
) -> ImportResult {
    let result = match kind {
        "raindrop" => raindrop_import::run_import(state, csv_bytes, concurrency, cancel, |event| {
            apply_raindrop_progress_only(&event);
        })
        .await
        .map(|_| ()),
        "article_csv" => article_csv::run_import(state, csv_bytes, concurrency, cancel, |event| {
            apply_article_progress_only(&event);
        })
        .await
        .map(|_| ()),
        other => Err(AppError::Internal(format!("unknown import kind: {other}"))),
    };

    match result {
        Ok(()) => ImportResult::ok(),
        Err(error) => {
            let mut snapshot = IMPORT_PROGRESS
                .get_or_init(Default::default)
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_default();
            snapshot.finished = true;
            snapshot.error = Some(error.to_string());
            set_progress(snapshot);
            ImportResult::err(error)
        }
    }
}

fn apply_raindrop_event(event: &raindrop_import::ImportEvent, app: &tauri::AppHandle) {
    use crate::events;
    apply_raindrop_progress_only(event);
    match event {
        raindrop_import::ImportEvent::Started { total } => events::emit_import_started(app, *total),
        raindrop_import::ImportEvent::Progress(progress) => {
            events::emit_import_progress(app, progress)
        }
        raindrop_import::ImportEvent::LibraryChanged => events::emit_articles_changed(app),
        raindrop_import::ImportEvent::Finished(finished) => {
            events::emit_import_finished(app, finished)
        }
    }
}

fn apply_raindrop_progress_only(event: &raindrop_import::ImportEvent) {
    match event {
        raindrop_import::ImportEvent::Started { total } => set_progress(ProgressSnapshot {
            total: *total,
            ..Default::default()
        }),
        raindrop_import::ImportEvent::Progress(progress) => set_progress(ProgressSnapshot {
            total: progress.total,
            processed: progress.processed,
            imported: progress.imported,
            skipped_duplicate: progress.skipped_duplicate,
            failed: progress.failed,
            finished: false,
            cancelled: false,
            error: None,
        }),
        raindrop_import::ImportEvent::LibraryChanged => {}
        raindrop_import::ImportEvent::Finished(finished) => set_progress(ProgressSnapshot {
            total: finished.total,
            processed: finished.total,
            imported: finished.imported,
            skipped_duplicate: finished.skipped_duplicate,
            failed: finished.failed.len() as u32,
            finished: true,
            cancelled: finished.cancelled,
            error: None,
        }),
    }
}

fn apply_article_event(event: &article_csv::ImportEvent, app: &tauri::AppHandle) {
    use crate::events;
    apply_article_progress_only(event);
    match event {
        article_csv::ImportEvent::Started { total } => {
            events::emit_article_import_started(app, *total)
        }
        article_csv::ImportEvent::Progress(progress) => {
            events::emit_article_import_progress(app, progress)
        }
        article_csv::ImportEvent::LibraryChanged => events::emit_articles_changed(app),
        article_csv::ImportEvent::Finished(finished) => {
            events::emit_article_import_finished(app, finished)
        }
    }
}

fn apply_article_progress_only(event: &article_csv::ImportEvent) {
    match event {
        article_csv::ImportEvent::Started { total } => set_progress(ProgressSnapshot {
            total: *total,
            ..Default::default()
        }),
        article_csv::ImportEvent::Progress(progress) => set_progress(ProgressSnapshot {
            total: progress.total,
            processed: progress.processed,
            imported: progress.imported,
            skipped_duplicate: progress.skipped_duplicate,
            failed: progress.failed,
            finished: false,
            cancelled: false,
            error: None,
        }),
        article_csv::ImportEvent::LibraryChanged => {}
        article_csv::ImportEvent::Finished(finished) => set_progress(ProgressSnapshot {
            total: finished.total,
            processed: finished.total,
            imported: finished.imported,
            skipped_duplicate: finished.skipped_duplicate,
            failed: finished.failed.len() as u32,
            finished: true,
            cancelled: finished.cancelled,
            error: None,
        }),
    }
}

/// Builds a throwaway `AppState` — DB pool, SSRF-guarded HTTP client,
/// migrated schema — for when this process has no already-running Tauri
/// app instance to reuse (the app was killed and `WorkManager` restarted
/// the job in a fresh process). Mirrors `share_intent::run_capture`'s
/// identical fallback, just assembling a real `AppState` rather than
/// calling `direct_link::capture_and_store` directly, since
/// `raindrop_import::run_import`/`article_csv::run_import` both take
/// `&AppState` (only ever touching its `pool`/`http_client`/`data_dir`
/// fields — every other field below is an inert placeholder, exactly
/// like each importer's own unit test `build_state` helper).
async fn build_standalone_state(data_dir: &std::path::Path) -> Result<AppState, String> {
    tokio::fs::create_dir_all(data_dir)
        .await
        .map_err(|e| e.to_string())?;

    let pool = db::build_pool(&data_dir.join("legere.db")).map_err(|e| e.to_string())?;
    {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            db::schema::migrate(&mut conn).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())??;
    }

    Ok(AppState {
        pool,
        http_client: build_client(),
        data_dir: data_dir.to_path_buf(),
        autosync_handle: tokio::sync::Mutex::new(None),
        remote_sync_handle: tokio::sync::Mutex::new(None),
        last_foreground_sync: StdMutex::new(None),
        import_cancel: tokio::sync::Mutex::new(None),
        article_import_cancel: tokio::sync::Mutex::new(None),
        remote_sync_cancel: tokio::sync::Mutex::new(None),
        capture_jobs: Default::default(),
        remote_sync_client_cache: Default::default(),
        #[cfg(feature = "apk-self-update")]
        pending_android_update: Default::default(),
    })
}

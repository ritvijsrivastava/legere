//! JNI entrypoint backing Android's share-intent flow: sharing a URL from
//! another app (browser, feed reader, etc.) into Legere runs entirely in
//! the background, no window ever shown.
//!
//! The Android side (`gen/android/app/src/main/java/.../ShareActivity.kt`,
//! `ShareWorker.kt`, `NativeCapture.kt`) is a trampoline
//! `Activity` — invisible, `excludeFromRecents`/`noHistory` — that pulls a
//! URL out of the incoming `ACTION_SEND` intent, posts an initial "Saving
//! article..." notification, hands the URL off to an expedited
//! `WorkManager` job, and finishes immediately without ever inflating a
//! layout. The worker calls [`Java_com_ritvijsrivastava_legere_NativeCapture_captureSharedUrl`]
//! below and updates that same notification once it returns.
//!
//! Prefers routing the capture through this app's own already-running
//! `AppState`/async runtime when one exists in this process
//! (`try_capture_via_running_app`, using `crate::GLOBAL_APP_HANDLE`) —
//! the correct data dir for free, and an instant `articles:changed`
//! refresh if the library view happens to already be open. Falls back to
//! a fully standalone DB pool + HTTP client + Tokio runtime
//! (`run_capture`) when there's no running instance to reuse — a share
//! is very often the *only* thing that happens in this app process:
//! Android starts the process for the `WorkManager` job alone,
//! `MainActivity.onCreate` (and therefore `app.manage(AppState)`) never
//! runs. That fallback also migrates the database if needed (first-ever
//! launch could plausibly be a share, before the user has ever opened
//! the app) and runs the exact same `capture::capture_local` pipeline
//! every other ingestion path uses, via
//! `sources::direct_link::capture_and_store`. Running concurrently with a
//! live app instance (i.e. if the fallback path somehow raced the live
//! one) is still safe either way: both just open their own connection to
//! the same `legere.db`, and WAL mode + `busy_timeout`
//! (`db::pool::build_pool`) already has to handle that.
//!
//! Desktop has no equivalent — there's no OS-level share-sheet concept on
//! Linux; this whole module is only compiled for `target_os = "android"`
//! (gated at the `mod share_intent;` declaration in `lib.rs`, not here).

use std::path::PathBuf;

use jni::EnvUnowned;
use jni::errors::LogErrorAndDefault;
use jni::objects::{JClass, JObject, JString};
use serde::Serialize;
use tauri::Manager;

use crate::capture::fetch::build_client;
use crate::db;
use crate::sources::direct_link;

#[derive(Serialize)]
struct ShareCaptureResult {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl ShareCaptureResult {
    fn ok(title: String) -> Self {
        Self {
            ok: true,
            title: Some(title),
            error: None,
        }
    }

    fn err(error: impl ToString) -> Self {
        Self {
            ok: false,
            title: None,
            error: Some(error.to_string()),
        }
    }

    /// Always produces valid JSON — every field is a plain `String`/`bool`,
    /// nothing that can fail to serialize.
    fn to_json(&self) -> String {
        serde_json::to_string(self).expect("ShareCaptureResult always serializes")
    }
}

/// Called from `NativeCapture.captureSharedUrl` (a Kotlin `object`, hence
/// the `JClass` rather than an instance `JObject` as the second argument)
/// on a `WorkManager` background thread. Blocks that thread until the
/// capture finishes — expected to take anywhere from under a second to
/// tens of seconds for an image-heavy article, which is exactly the shape
/// of work an expedited `WorkManager` job is meant to own.
///
/// `context` is `applicationContext` from the calling `Worker`, needed for
/// the same `rustls-platform-verifier` JNI handoff `MainActivity.initTls`
/// normally does (see `mobile_tls`'s module docs for why it can't be
/// assumed to have already happened). `data_dir` is
/// `{applicationContext.dataDir}/legere` — the same path Tauri's own
/// `app_local_data_dir()` resolves to (`Context.getDataDir()`, *not*
/// `getFilesDir()` — see `ShareWorker.kt`) — only actually used by the
/// `run_capture` fallback below; the preferred
/// `try_capture_via_running_app` path reads the real `AppState`'s own
/// `data_dir` instead and ignores this argument entirely.
///
/// Returns a JSON string: `{"ok":true,"title":"..."}` on success,
/// `{"ok":false,"error":"..."}` otherwise. Never throws a Java exception —
/// failures are reported in the JSON so the worker can update its
/// notification with a real (if terse) reason rather than a generic
/// failure.
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_com_ritvijsrivastava_legere_NativeCapture_captureSharedUrl<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    context: JObject<'local>,
    data_dir: JString<'local>,
    url: JString<'local>,
) -> jni::sys::jstring {
    let inputs = env
        .with_env(|env| -> jni::errors::Result<(String, String)> {
            if let Err(error) = crate::mobile_tls::init_tls(env, context) {
                tracing::error!(%error, "share-intent capture: failed to init TLS context");
            }
            let data_dir = data_dir.try_to_string(&env)?;
            let url = url.try_to_string(&env)?;
            Ok((data_dir, url))
        })
        .resolve::<LogErrorAndDefault>();

    let (data_dir, url) = inputs;
    let result = if url.is_empty() {
        ShareCaptureResult::err("missing arguments")
    } else if let Some(result) = try_capture_via_running_app(&url) {
        result
    } else if data_dir.is_empty() {
        ShareCaptureResult::err("missing arguments")
    } else {
        run_capture(PathBuf::from(data_dir), url)
    };
    let json = result.to_json();

    env.with_env(|env| -> jni::errors::Result<jni::sys::jstring> {
        Ok(env.new_string(&json)?.into_raw())
    })
    .resolve::<LogErrorAndDefault>()
}

/// If this process already has a running Tauri app instance — the user
/// had Legere open, even just backgrounded, when the share completed —
/// routes the capture through its real `AppState` instead of building a
/// throwaway one, and fires `articles:changed` so an already-open
/// library view updates immediately rather than only on next refetch.
/// `None` (meaning "fall back to `run_capture`") if there's no running
/// instance in this process to reuse.
fn try_capture_via_running_app(url: &str) -> Option<ShareCaptureResult> {
    let app = crate::GLOBAL_APP_HANDLE.get()?.clone();
    let url = url.to_string();
    let result = {
        let app = app.clone();
        tauri::async_runtime::block_on(async move {
            let state = app.state::<crate::state::AppState>();
            direct_link::capture_and_store(&state.pool, &state.http_client, &state.data_dir, &url)
                .await
        })
    };
    Some(match result {
        Ok(article) => {
            crate::events::emit_articles_changed(&app);
            ShareCaptureResult::ok(article.title)
        }
        Err(error) => ShareCaptureResult::err(error),
    })
}

/// Builds a throwaway DB pool + SSRF-guarded HTTP client and runs
/// `capture::capture_local` via `direct_link::capture_and_store`, blocking
/// the calling thread on a freshly built single-purpose Tokio runtime.
/// This only ever runs once per share, so a fresh runtime per call is
/// simpler than keeping one alive for the rest of the process's life —
/// there's no ambient Tauri async runtime to reuse outside of `AppState`.
fn run_capture(data_dir: PathBuf, url: String) -> ShareCaptureResult {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => return ShareCaptureResult::err(error),
    };
    runtime.block_on(async move {
        if let Err(error) = tokio::fs::create_dir_all(&data_dir).await {
            return ShareCaptureResult::err(error);
        }

        let pool = match db::build_pool(&data_dir.join("legere.db")) {
            Ok(pool) => pool,
            Err(error) => return ShareCaptureResult::err(error),
        };

        // Mirrors `lib.rs`'s own startup migration — a share can be the
        // very first thing this app ever does on a device, before the
        // user has opened it once, so the schema may not exist yet.
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
            Ok(Err(message)) => return ShareCaptureResult::err(message),
            Err(join_error) => return ShareCaptureResult::err(join_error),
        }

        let client = build_client();
        match direct_link::capture_and_store(&pool, &client, &data_dir, &url).await {
            Ok(article) => ShareCaptureResult::ok(article.title),
            Err(error) => ShareCaptureResult::err(error),
        }
    })
}

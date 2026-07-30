mod capture;
mod commands;
mod db;
mod error;
mod events;
mod gc;
mod mobile_tls;
mod models;
mod sources;
mod state;
mod sync;
#[cfg(test)]
mod test_support;
mod zim_server;

use std::time::Duration;

use tauri::Manager;
use tokio::sync::Mutex;

use state::AppState;

const AUTOSYNC_INTERVAL: Duration = Duration::from_secs(15 * 60);
/// There's no background autosync on Android (no WorkManager integration
/// in the MVP — see `state::AppState::last_foreground_sync`), so a
/// resumed app only gets a fresh sync if it's been a while.
#[cfg(mobile)]
const FOREGROUND_SYNC_MIN_INTERVAL: Duration = Duration::from_secs(5 * 60);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());
    // See `capture::render_android`/`page_capture_plugin`'s own docs —
    // drives an off-layout Android `WebView` for capture, mirroring
    // `capture::render_linux` on desktop Linux.
    #[cfg(target_os = "android")]
    {
        builder = builder.plugin(page_capture_plugin::init());
    }

    builder
        // Serves archived pages out of an article's own ZIM file — see
        // `zim_server`'s module docs. Runs the actual read on a plain OS
        // thread, matching Tauri's own documented pattern for this API: a
        // protocol handler can be invoked from a webview thread with no
        // Tokio runtime entered on it, so `tokio::spawn`/`spawn_blocking`
        // aren't safe to call here the way they are from inside a command.
        .register_asynchronous_uri_scheme_protocol("zim", |ctx, request, responder| {
            let app_handle = ctx.app_handle().clone();
            let path = request.uri().path().to_string();
            std::thread::spawn(move || {
                let state = app_handle.state::<AppState>();
                responder.respond(zim_server::serve(state.inner(), &path));
            });
        })
        .setup(|app| {
            let data_dir = app.path().app_local_data_dir()?.join("legere");
            std::fs::create_dir_all(&data_dir)?;

            let pool = db::build_pool(&data_dir.join("legere.db"))?;
            {
                let mut conn = pool.get()?;
                db::schema::migrate(&mut conn)?;
            }

            let autosync_enabled = {
                let conn = pool.get()?;
                db::queries::get_settings(&conn)?.autosync
            };

            let state = AppState {
                pool,
                http_client: capture::fetch::build_client(),
                data_dir,
                autosync_handle: Mutex::new(None),
                zim_cache: zim_server::ZimCache::new(),
                last_foreground_sync: std::sync::Mutex::new(None),
            };
            app.manage(state);

            if autosync_enabled {
                let handle = sync::spawn_autosync(app.handle().clone(), AUTOSYNC_INTERVAL);
                let app_state = app.state::<AppState>();
                tauri::async_runtime::block_on(async {
                    *app_state.autosync_handle.lock().await = Some(handle);
                });
                // `spawn_autosync` fires an immediate sync on its own; on
                // Android, the initial `RunEvent::Resumed` (part of normal
                // cold-start activity lifecycle, not just backgrounding)
                // would otherwise race it into a redundant second sync —
                // see `last_foreground_sync`'s doc comment.
                #[cfg(mobile)]
                {
                    *app_state.last_foreground_sync.lock().unwrap() = Some(std::time::Instant::now());
                }
            }

            // One-shot cleanup of files orphaned by crashes or removal
            // paths that predate `delete_article`'s own GC. Spawned
            // rather than awaited so it never delays startup — see
            // `gc`'s module docs.
            let app_handle_for_gc = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let state = app_handle_for_gc.state::<AppState>();
                gc::sweep_orphaned_files(&state).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::articles::list_articles,
            commands::articles::get_article,
            commands::articles::mark_read,
            commands::articles::toggle_favorite,
            commands::articles::save_reading_progress,
            commands::articles::delete_article,
            commands::articles::recapture_article,
            commands::articles::add_direct_link_article,
            commands::sources::list_sources,
            commands::sources::add_source,
            commands::sources::toggle_source_pause,
            commands::sources::remove_source,
            commands::sources::sync_source,
            commands::sources::sync_all,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::system::get_data_dir,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // Unused on desktop, where autosync's own interval loop keeps
            // running regardless of window focus.
            let _ = (&app_handle, &event);

            #[cfg(mobile)]
            if let tauri::RunEvent::Resumed = event {
                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app_handle.state::<AppState>();
                    let should_sync = {
                        let mut last = state.last_foreground_sync.lock().unwrap();
                        let now = std::time::Instant::now();
                        let should_sync = match *last {
                            Some(t) => now.duration_since(t) > FOREGROUND_SYNC_MIN_INTERVAL,
                            None => true,
                        };
                        if should_sync {
                            *last = Some(now);
                        }
                        should_sync
                    };
                    if should_sync {
                        sync::sync_all_sources(&app_handle, &state).await;
                    }
                });
            }
        });
}

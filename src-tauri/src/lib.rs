mod capture;
mod commands;
mod db;
mod models;
mod sources;
mod state;
mod sync;
#[cfg(test)]
mod test_support;

use std::time::Duration;

use tauri::Manager;
use tokio::sync::Mutex;

use state::AppState;

const AUTOSYNC_INTERVAL: Duration = Duration::from_secs(15 * 60);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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
            };
            app.manage(state);

            if autosync_enabled {
                let handle = sync::spawn_autosync(app.handle().clone(), AUTOSYNC_INTERVAL);
                let app_state = app.state::<AppState>();
                tauri::async_runtime::block_on(async {
                    *app_state.autosync_handle.lock().await = Some(handle);
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::articles::list_articles,
            commands::articles::get_article,
            commands::articles::mark_read,
            commands::articles::toggle_favorite,
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

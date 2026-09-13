use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::ApkInstaller;
#[cfg(mobile)]
use mobile::ApkInstaller;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the apk-installer APIs.
pub trait ApkInstallerExt<R: Runtime> {
    fn apk_installer(&self) -> &ApkInstaller<R>;
}

impl<R: Runtime, T: Manager<R>> crate::ApkInstallerExt<R> for T {
    fn apk_installer(&self) -> &ApkInstaller<R> {
        self.state::<ApkInstaller<R>>().inner()
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("apk-installer")
        .invoke_handler(tauri::generate_handler![
            commands::can_request_installs,
            commands::request_install_permission,
            commands::install,
        ])
        .setup(|app, api| {
            #[cfg(mobile)]
            let apk_installer = mobile::init(app, api)?;
            #[cfg(desktop)]
            let apk_installer = desktop::init(app, api)?;
            app.manage(apk_installer);
            Ok(())
        })
        .build()
}

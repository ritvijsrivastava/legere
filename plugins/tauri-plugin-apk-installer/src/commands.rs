use tauri::{command, AppHandle, Runtime};

use crate::ApkInstallerExt;
use crate::Result;

/// Whether the OS will let this app launch the package installer
/// (Android 8+ requires the user to explicitly enable "install unknown apps").
#[command]
pub(crate) async fn can_request_installs<R: Runtime>(app: AppHandle<R>) -> Result<bool> {
    app.apk_installer().can_request_installs()
}

/// Send the user to the system settings screen to grant install permission.
#[command]
pub(crate) async fn request_install_permission<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    app.apk_installer().request_install_permission()
}

/// Launch the Android package installer for the APK at `path`.
#[command]
pub(crate) async fn install<R: Runtime>(app: AppHandle<R>, path: String) -> Result<()> {
    app.apk_installer().install(path)
}

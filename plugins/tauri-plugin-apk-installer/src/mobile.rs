use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use crate::models::*;

// initializes the Kotlin plugin class
pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> crate::Result<ApkInstaller<R>> {
    #[cfg(target_os = "android")]
    let handle = api.register_android_plugin(
        "com.ritvijsrivastava.legere.apkinstaller",
        "ApkInstallerPlugin",
    )?;
    Ok(ApkInstaller(handle))
}

/// Access to the apk-installer APIs.
pub struct ApkInstaller<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> ApkInstaller<R> {
    pub fn can_request_installs(&self) -> crate::Result<bool> {
        let res: CanRequestInstallsResponse = self.0.run_mobile_plugin("canRequestInstalls", ())?;
        Ok(res.value)
    }

    pub fn request_install_permission(&self) -> crate::Result<()> {
        self.0
            .run_mobile_plugin("requestInstallPermission", ())
            .map_err(Into::into)
    }

    pub fn install(&self, path: String) -> crate::Result<()> {
        self.0
            .run_mobile_plugin("install", InstallRequest { path })
            .map_err(Into::into)
    }
}

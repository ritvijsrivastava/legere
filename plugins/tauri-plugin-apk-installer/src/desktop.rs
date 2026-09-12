use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

pub fn init<R: Runtime, C: DeserializeOwned>(
  app: &AppHandle<R>,
  _api: PluginApi<R, C>,
) -> crate::Result<ApkInstaller<R>> {
  Ok(ApkInstaller(app.clone()))
}

/// Desktop stub — this plugin is only actually used on Android; the app never
/// depends on this crate when building for desktop targets.
pub struct ApkInstaller<R: Runtime>(AppHandle<R>);

impl<R: Runtime> ApkInstaller<R> {
  pub fn can_request_installs(&self) -> crate::Result<bool> {
    Ok(false)
  }

  pub fn request_install_permission(&self) -> crate::Result<()> {
    Ok(())
  }

  pub fn install(&self, _path: String) -> crate::Result<()> {
    Ok(())
  }
}

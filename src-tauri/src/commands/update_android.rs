//! Android self-update. tauri-plugin-updater doesn't support mobile, so this
//! reads the same `latest.json` manifest desktop uses (see the CI step that
//! patches an `android-aarch64` entry into it), then hands the downloaded APK
//! to the local apk-installer plugin for the actual install-intent +
//! permission dance.
//!
//! Both the manifest and the APK are fetched from `api.github.com`
//! (`Accept: application/vnd.github...` or `application/octet-stream`)
//! rather than the `github.com/.../releases/latest/download/...` convenience
//! URLs, since that's the URL shape `release.assets[].url` gives us. The
//! GitHub API also hard-requires a `User-Agent` header on every request.

use std::collections::HashMap;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager, State, ipc::Channel};
use tauri_plugin_apk_installer::ApkInstallerExt;

use super::update::{OWNER, REPO, USER_AGENT};
use crate::error::AppError;
use crate::models::UpdateInfo;
use crate::state::AppState;

#[derive(Deserialize)]
struct Release {
    assets: Vec<ReleaseAsset>,
}

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    url: String,
}

#[derive(Deserialize)]
struct Manifest {
    version: String,
    notes: Option<String>,
    platforms: HashMap<String, PlatformEntry>,
}

#[derive(Deserialize, Clone)]
struct PlatformEntry {
    url: String,
    sha256: Option<String>,
}

/// Update found by `android_check_for_update`, consumed by
/// `android_download_and_install`.
#[derive(Clone)]
pub struct AndroidUpdate {
    pub url: String,
    pub sha256: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(tag = "event", content = "data", rename_all_fields = "camelCase")]
pub enum DownloadProgress {
    Started { content_length: Option<u64> },
    Progress { chunk_length: usize },
    Finished,
}

#[tauri::command]
pub async fn android_check_for_update(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<UpdateInfo>, AppError> {
    let client = reqwest::Client::new();

    let release: Release = client
        .get(format!(
            "https://api.github.com/repos/{OWNER}/{REPO}/releases/latest"
        ))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| AppError::Network(e.to_string()))?
        .error_for_status()
        .map_err(|e| AppError::Network(e.to_string()))?
        .json()
        .await
        .map_err(|e| AppError::Network(e.to_string()))?;

    let manifest_asset_url = release
        .assets
        .iter()
        .find(|a| a.name == "latest.json")
        .map(|a| a.url.clone())
        .ok_or_else(|| {
            AppError::Internal("No latest.json found in the latest release.".to_string())
        })?;

    let manifest: Manifest = client
        .get(&manifest_asset_url)
        .header("Accept", "application/octet-stream")
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| AppError::Network(e.to_string()))?
        .error_for_status()
        .map_err(|e| AppError::Network(e.to_string()))?
        .json()
        .await
        .map_err(|e| AppError::Network(e.to_string()))?;

    let current = app.package_info().version.clone();
    let latest =
        semver::Version::parse(&manifest.version).map_err(|e| AppError::Internal(e.to_string()))?;

    if latest <= current {
        *state.pending_android_update.lock().await = None;
        return Ok(None);
    }

    let platform = manifest.platforms.get("android-aarch64").ok_or_else(|| {
        AppError::Internal("No Android build found in the latest release.".to_string())
    })?;

    *state.pending_android_update.lock().await = Some(AndroidUpdate {
        url: platform.url.clone(),
        sha256: platform.sha256.clone(),
    });

    Ok(Some(UpdateInfo {
        version: manifest.version,
        notes: manifest.notes,
        date: None,
    }))
}

#[tauri::command]
pub async fn android_download_and_install(
    app: AppHandle,
    state: State<'_, AppState>,
    on_progress: Channel<DownloadProgress>,
) -> Result<(), AppError> {
    let update = state
        .pending_android_update
        .lock()
        .await
        .take()
        .ok_or_else(|| {
            AppError::Internal("No pending update. Check for updates first.".to_string())
        })?;

    let installer = app.apk_installer();
    if !installer
        .can_request_installs()
        .map_err(|e| AppError::Internal(e.to_string()))?
    {
        installer
            .request_install_permission()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        if !installer
            .can_request_installs()
            .map_err(|e| AppError::Internal(e.to_string()))?
        {
            return Err(AppError::Internal(
                "Install permission not granted. Enable \"install unknown apps\" for Legere \
                 and try again."
                    .to_string(),
            ));
        }
    }

    let response = reqwest::Client::new()
        .get(&update.url)
        .header("Accept", "application/octet-stream")
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| AppError::Network(e.to_string()))?
        .error_for_status()
        .map_err(|e| AppError::Network(e.to_string()))?;

    let _ = on_progress.send(DownloadProgress::Started {
        content_length: response.content_length(),
    });

    let mut hasher = Sha256::new();
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::Network(e.to_string()))?;
        hasher.update(&chunk);
        bytes.extend_from_slice(&chunk);
        let _ = on_progress.send(DownloadProgress::Progress {
            chunk_length: chunk.len(),
        });
    }

    if let Some(expected) = &update.sha256 {
        let actual = format!("{:x}", hasher.finalize());
        if &actual != expected {
            return Err(AppError::Internal(
                "Downloaded APK failed its integrity check.".to_string(),
            ));
        }
    }

    // The app's FileProvider (gen/android) covers the whole cache dir, so any
    // path under here is safe to hand to the installer.
    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    std::fs::create_dir_all(&cache_dir).map_err(|e| AppError::Internal(e.to_string()))?;
    let apk_path = cache_dir.join("legere-update.apk");
    std::fs::write(&apk_path, &bytes).map_err(|e| AppError::Internal(e.to_string()))?;

    let _ = on_progress.send(DownloadProgress::Finished);

    installer
        .install(apk_path.to_string_lossy().to_string())
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(())
}

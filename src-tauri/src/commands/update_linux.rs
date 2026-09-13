//! Linux .deb/.rpm self-update.
//!
//! tauri-plugin-updater's Linux support is AppImage-only (it self-updates by
//! replacing the running file in place via the `APPIMAGE` env var), so a
//! .deb/.rpm install needs a fully custom path — same reasoning as
//! `update_android.rs`, and following the same shape: read the same
//! `latest.json` manifest desktop uses (see the CI step that patches
//! `linux-x86_64-deb`/`linux-x86_64-rpm` entries into it), download+verify
//! the package, then elevate via `pkexec` to hand it to `apt-get`/`dnf` for
//! an in-place install (chosen over `dpkg -i`/`rpm -Uvh` so a future
//! dependency bump doesn't produce a half-broken install — trading in a
//! requirement for reachable apt/dnf repos as an accepted risk).

use serde::{Deserialize, Serialize};

/// Update found by `linux_check_for_update`, consumed by `linux_install_update`.
#[derive(Clone)]
pub struct LinuxUpdate {
    pub url: String,
    pub sha256: Option<String>,
    pub kind: LinuxPackageKind,
}

#[derive(Clone, Copy)]
pub enum LinuxPackageKind {
    Deb,
    Rpm,
}

impl LinuxPackageKind {
    /// Key this package's entry is stored under in `latest.json`'s `platforms` map.
    fn platform_key(self) -> &'static str {
        match self {
            LinuxPackageKind::Deb => "linux-x86_64-deb",
            LinuxPackageKind::Rpm => "linux-x86_64-rpm",
        }
    }

    fn package_ext(self) -> &'static str {
        match self {
            LinuxPackageKind::Deb => "deb",
            LinuxPackageKind::Rpm => "rpm",
        }
    }
}

#[cfg(target_os = "linux")]
#[derive(Deserialize)]
struct Release {
    assets: Vec<ReleaseAsset>,
}

#[cfg(target_os = "linux")]
#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    url: String,
}

#[cfg(target_os = "linux")]
#[derive(Deserialize)]
struct Manifest {
    version: String,
    notes: Option<String>,
    platforms: std::collections::HashMap<String, PlatformEntry>,
}

#[cfg(target_os = "linux")]
#[derive(Deserialize, Clone)]
struct PlatformEntry {
    url: String,
    sha256: Option<String>,
}

/// Download/install progress, streamed to the frontend. No `Finished`
/// variant here (unlike the desktop/Android equivalents) — once the download
/// and checksum check are done, this jumps straight to `Installing`, since
/// the frontend renders `Finished` as "Installed — relaunching…" and there's
/// still a `pkexec` prompt and an `apt`/`dnf` run ahead at that point.
#[derive(Clone, Serialize)]
#[serde(tag = "event", content = "data", rename_all_fields = "camelCase")]
pub enum DownloadProgress {
    Started { content_length: Option<u64> },
    Progress { chunk_length: usize },
    Installing,
}

#[cfg(target_os = "linux")]
mod imp {
    use std::io::ErrorKind;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use std::process::Command;

    use futures_util::StreamExt;
    use sha2::{Digest, Sha256};
    use tauri::{AppHandle, State, ipc::Channel};

    use super::{DownloadProgress, LinuxPackageKind, LinuxUpdate, Manifest, Release};
    use crate::commands::update::{OWNER, REPO, USER_AGENT, read_token};
    use crate::error::AppError;
    use crate::models::UpdateInfo;
    use crate::state::AppState;

    /// `None` if this doesn't look like a package-manager-installed Legere at
    /// all (a dev build, or a binary copied out of a bundle) — the only case
    /// where the frontend should still show a "couldn't detect how this was
    /// installed" note. `Some("appimage")` is reported here too so the
    /// frontend can positively recognize it and *not* show that note (that
    /// install type already self-updates fine via tauri-plugin-updater).
    fn detect_linux_install_kind() -> Option<&'static str> {
        if std::env::var_os("APPIMAGE").is_some() {
            return Some("appimage");
        }

        // `dpkg -s legere` alone would false-positive on a package that was
        // `apt remove`'d but left its config behind (status stays "ok
        // config-files", not "not-installed"), so check the status field
        // directly instead.
        if let Ok(output) = Command::new("dpkg-query")
            .args(["-W", "-f=${db:Status-Status}", "legere"])
            .output()
        {
            if output.status.success()
                && String::from_utf8_lossy(&output.stdout).trim() == "installed"
            {
                return Some("deb");
            }
        }

        // `.status()` inherits the parent's stdio by default, so without
        // this `rpm -q` on a system where legere isn't rpm-installed prints
        // its own "package legere is not installed" straight to our stderr.
        if let Ok(status) = Command::new("rpm")
            .args(["-q", "legere"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
        {
            if status.success() {
                return Some("rpm");
            }
        }

        None
    }

    #[tauri::command]
    pub async fn linux_install_kind() -> Result<Option<String>, AppError> {
        Ok(detect_linux_install_kind().map(str::to_string))
    }

    #[tauri::command]
    pub async fn linux_check_for_update(
        app: AppHandle,
        state: State<'_, AppState>,
    ) -> Result<Option<UpdateInfo>, AppError> {
        let kind = match detect_linux_install_kind() {
            Some("deb") => LinuxPackageKind::Deb,
            Some("rpm") => LinuxPackageKind::Rpm,
            _ => {
                return Err(AppError::Internal(
                    "Not a .deb or .rpm install.".to_string(),
                ));
            }
        };

        let token = read_token(&app)?;
        let client = reqwest::Client::new();

        let release: Release = client
            .get(format!(
                "https://api.github.com/repos/{OWNER}/{REPO}/releases/latest"
            ))
            .header("Authorization", format!("Bearer {token}"))
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
            .header("Authorization", format!("Bearer {token}"))
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
        let latest = semver::Version::parse(&manifest.version)
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if latest <= current {
            *state.pending_linux_update.lock().await = None;
            return Ok(None);
        }

        let platform = manifest.platforms.get(kind.platform_key()).ok_or_else(|| {
            AppError::Internal("No matching Linux build found in the latest release.".to_string())
        })?;

        *state.pending_linux_update.lock().await = Some(LinuxUpdate {
            url: platform.url.clone(),
            sha256: platform.sha256.clone(),
            kind,
        });

        Ok(Some(UpdateInfo {
            version: manifest.version,
            notes: manifest.notes,
            date: None,
        }))
    }

    #[tauri::command]
    pub async fn linux_install_update(
        state: State<'_, AppState>,
        app: AppHandle,
        on_progress: Channel<DownloadProgress>,
    ) -> Result<(), AppError> {
        // Captured before pkexec runs, while this process's exe path still
        // resolves to a live inode — apt/dnf replace /usr/bin/legere by
        // unpacking to a temp name and renaming over it, which unlinks the
        // *running* process's inode. After that, `current_exe()` would
        // resolve to a "(deleted)"-suffixed path that can't be relaunched.
        let current_exe = std::env::current_exe().map_err(|e| AppError::Internal(e.to_string()))?;

        let update = state
            .pending_linux_update
            .lock()
            .await
            .take()
            .ok_or_else(|| {
                AppError::Internal("No pending update. Check for updates first.".to_string())
            })?;

        let token = read_token(&app)?;
        let response = reqwest::Client::new()
            .get(&update.url)
            .header("Authorization", format!("Bearer {token}"))
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
                    "Downloaded package failed its integrity check.".to_string(),
                ));
            }
        }

        // /tmp, not app_cache_dir(): apt/dpkg read the local file as the
        // low-privilege _apt user, which may not be able to traverse into the
        // app's ~/.cache directory.
        let pkg_path =
            std::env::temp_dir().join(format!("legere-update.{}", update.kind.package_ext()));
        std::fs::write(&pkg_path, &bytes).map_err(|e| AppError::Internal(e.to_string()))?;
        std::fs::set_permissions(&pkg_path, std::fs::Permissions::from_mode(0o644))
            .map_err(|e| AppError::Internal(e.to_string()))?;

        let _ = on_progress.send(DownloadProgress::Installing);

        let (program, args): (&str, Vec<String>) = match update.kind {
            // apt-get (not `apt`, whose CLI is explicitly not stable for
            // scripting) resolves and installs any new dependencies from
            // configured repos, unlike a plain `dpkg -i`.
            LinuxPackageKind::Deb => (
                "apt-get",
                vec![
                    "install".to_string(),
                    "-y".to_string(),
                    pkg_path.to_string_lossy().to_string(),
                ],
            ),
            LinuxPackageKind::Rpm => {
                let dnf_present = Path::new("/usr/bin/dnf").exists();
                (
                    if dnf_present { "dnf" } else { "yum" },
                    vec![
                        "install".to_string(),
                        "-y".to_string(),
                        pkg_path.to_string_lossy().to_string(),
                    ],
                )
            }
        };

        let pkg_path_display = pkg_path.display().to_string();
        let install_result = tauri::async_runtime::spawn_blocking(move || {
            Command::new("pkexec").arg(program).args(&args).status()
        })
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        match install_result {
            Ok(status) if status.success() => {}
            Ok(_) => {
                return Err(AppError::Internal(format!(
                    "Install was cancelled or failed. You can update manually from the downloaded \
                     package: {pkg_path_display}"
                )));
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {
                return Err(AppError::Internal(format!(
                    "pkexec is not available on this system. Install polkit, or update the package \
                     manually: {pkg_path_display}"
                )));
            }
            Err(e) => {
                return Err(AppError::Internal(format!(
                    "Failed to launch the installer: {e}"
                )));
            }
        }

        Command::new(&current_exe).spawn().map_err(|e| {
            AppError::Internal(format!(
                "Update installed, but couldn't relaunch automatically ({e}). Please restart Legere manually."
            ))
        })?;
        app.exit(0);

        Ok(())
    }
}

#[cfg(not(target_os = "linux"))]
mod imp {
    use tauri::{State, ipc::Channel};

    use super::DownloadProgress;
    use crate::error::AppError;
    use crate::models::UpdateInfo;
    use crate::state::AppState;

    #[tauri::command]
    pub async fn linux_install_kind() -> Result<Option<String>, AppError> {
        Ok(None)
    }

    #[tauri::command]
    pub async fn linux_check_for_update(
        _state: State<'_, AppState>,
    ) -> Result<Option<UpdateInfo>, AppError> {
        Ok(None)
    }

    #[tauri::command]
    pub async fn linux_install_update(
        _state: State<'_, AppState>,
        _on_progress: Channel<DownloadProgress>,
    ) -> Result<(), AppError> {
        Err(AppError::Internal(
            "Not supported on this platform.".to_string(),
        ))
    }
}

pub use imp::{linux_check_for_update, linux_install_kind, linux_install_update};

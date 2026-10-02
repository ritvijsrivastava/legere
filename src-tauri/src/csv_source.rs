//! Reads the bytes of a CSV path chosen via the frontend's file picker
//! (`@tauri-apps/plugin-dialog`'s `open()`), used by all four CSV import
//! entry points (`commands::import::{preview_raindrop_csv,import_raindrop_csv}`,
//! `commands::export::{preview_articles_csv,import_articles_csv}`).
//!
//! On desktop that picker always returns a real filesystem path, so this
//! is just `tokio::fs::read`. On Android it instead returns a SAF
//! `content://` URI — a reference good only via `ContentResolver`, which
//! plain `std::fs`/`tokio::fs` (used here and, more importantly, by
//! `import_intent::run_import`'s `WorkManager` job, possibly running in a
//! completely different process) has no way to resolve at all. Worse,
//! even if we read it immediately, a `content://` grant isn't guaranteed
//! to survive this app's own process dying and `WorkManager` restarting
//! the job later — exactly what `import_intent.rs` exists to make safe.
//!
//! So on Android, a `content://` path is copied once, up front, to a
//! stable file under this app's own private storage
//! (`{data_dir}/tmp/picked_import.csv`) via `import_intent::
//! copy_content_uri_to_path` (JNI call to `MainActivity.copyContentUriToFile`,
//! which does the actual `ContentResolver.openInputStream` copy in
//! Kotlin) — from then on every caller, including a `WorkManager` job in
//! a future process, just has a normal file to read, no special
//! permissions or platform awareness needed.
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// Resolves `path` to a real, stable filesystem path this app can read
/// from repeatedly (including from a future process) \u2014 the same input
/// `path` unchanged on every platform/path shape except an Android
/// `content://` URI, which is copied once to `{data_dir}/tmp/
/// picked_import.csv`. Used both by the preview/foreground-import path
/// (immediately followed by a single read) and, on Android, by
/// `commands::import::import_raindrop_csv`/`commands::export::
/// import_articles_csv` to get a real path to hand to `import_intent::enqueue`
/// *before* enqueueing \u2014 the `WorkManager` job that actually runs the
/// import must never be handed the original `content://` URI itself (see
/// this module's own doc comment for why that grant can't be trusted to
/// still be readable by the time a background job gets to it).
pub async fn resolve_csv_path(
    #[cfg_attr(not(target_os = "android"), allow(unused_variables))] data_dir: &Path,
    path: &str,
) -> Result<PathBuf, AppError> {
    #[cfg(target_os = "android")]
    {
        if path.starts_with("content://") {
            let dest = data_dir.join("tmp").join("picked_import.csv");
            tokio::fs::create_dir_all(dest.parent().expect("has a parent")).await?;
            let dest_str = dest.to_string_lossy().to_string();
            let uri = path.to_string();
            tokio::task::spawn_blocking(move || {
                crate::import_intent::copy_content_uri_to_path(&uri, &dest_str)
            })
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .map_err(AppError::Internal)?;
            return Ok(dest);
        }
    }

    Ok(PathBuf::from(path))
}

pub async fn read_csv_bytes(data_dir: &Path, path: &str) -> Result<Vec<u8>, AppError> {
    let resolved = resolve_csv_path(data_dir, path).await?;
    Ok(tokio::fs::read(&resolved).await?)
}

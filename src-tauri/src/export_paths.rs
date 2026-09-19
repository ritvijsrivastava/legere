//! Shared helper behind every "Export ..." command
//! (`commands::export::export_articles_csv`,
//! `commands::sources::export_sources_csv`): writes CSV text to a fixed,
//! discoverable location — `{data_dir}/exports/` — rather than forcing a
//! save-dialog on every export. The file is still fully shareable
//! afterward (Settings/Sources show a "reveal in folder" action backed by
//! `tauri-plugin-opener`, plus a plain save-dialog copy for handing it to
//! another app) — this just means the export always exists somewhere
//! predictable even if the user closes that dialog immediately.

use std::path::Path;

use chrono::Utc;

use crate::error::AppError;
use crate::models::ExportResult;

/// Writes `contents` to `{data_dir}/exports/legere-<prefix>-<timestamp>.csv`,
/// creating the `exports/` directory if needed, and returns everything
/// the frontend needs to show/reveal/re-share the result.
pub async fn write_export_csv(
    data_dir: &Path,
    prefix: &str,
    contents: String,
    row_count: usize,
) -> Result<ExportResult, AppError> {
    let exports_dir = data_dir.join("exports");
    tokio::fs::create_dir_all(&exports_dir).await?;

    let now = Utc::now();
    // Filesystem-safe, sortable, and collision-resistant enough for a
    // manually-triggered export (down to the second; two exports in the
    // same second would collide, an accepted edge case for a user-driven
    // action rather than a hot loop).
    let filename = format!("legere-{prefix}-{}.csv", now.format("%Y%m%d-%H%M%S"));
    let path = exports_dir.join(&filename);
    tokio::fs::write(&path, contents).await?;

    Ok(ExportResult {
        path: path.to_string_lossy().into_owned(),
        filename,
        row_count: row_count as u32,
        exported_at: now.to_rfc3339(),
    })
}

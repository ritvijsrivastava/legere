use std::path::Path;

use thiserror::Error;

use crate::capture::{self, CaptureError};
use crate::db::DbPool;
use crate::db::queries;
use crate::models::ArticleSummary;
use crate::state::AppState;

#[derive(Debug, Error)]
pub enum DirectLinkError {
    #[error(transparent)]
    Capture(#[from] CaptureError),
    #[error(transparent)]
    Db(#[from] rusqlite::Error),
    #[error(transparent)]
    Pool(#[from] r2d2::Error),
}

/// A one-shot local-capture-and-insert for a user-submitted URL. Unlike
/// RSS, this isn't tied to a recurring `sources` row — `source_id` is left
/// `NULL`.
pub async fn capture_direct_link(
    state: &AppState,
    url: &str,
) -> Result<ArticleSummary, DirectLinkError> {
    capture_and_store(&state.pool, &state.http_client, &state.data_dir, url).await
}

/// Lower-level than [`capture_direct_link`]: takes the capture pipeline's
/// three dependencies directly rather than a whole `AppState`, so it's
/// callable from contexts that never construct one — notably Android's
/// share-intent entrypoint (`share_intent.rs`), which runs inside a
/// WorkManager background task that may be the *only* thing running in the
/// process (a share can happen without the app's own Tauri runtime/`AppState`
/// ever having been started in this process instance).
pub async fn capture_and_store(
    pool: &DbPool,
    http_client: &reqwest::Client,
    data_dir: &Path,
    url: &str,
) -> Result<ArticleSummary, DirectLinkError> {
    let id = uuid::Uuid::new_v4().to_string();
    let output = capture::capture_local(http_client, data_dir, &id, url).await?;

    let conn = pool.get()?;
    let inserted = queries::insert_captured_article(&conn, &id, None, "direct", &output, &[])?;

    // `output.link`'s cleaned form was already saved under a different id
    // (the user re-submitted a URL they already have) — the fresh `id`
    // below was never actually inserted, so return the article that's
    // really stored under that link instead of a summary describing a
    // row that doesn't exist.
    if !inserted && let Some(existing) = queries::get_article_summary_by_link(&conn, &output.link)?
    {
        return Ok(existing);
    }

    Ok(ArticleSummary {
        id,
        title: output.title,
        source_type: "direct".to_string(),
        excerpt: output.excerpt,
        hero_image_path: output.hero_image_path,
        published_at: output.published_at,
        read_time_min: output.read_time_min,
        reading_state: "unread".to_string(),
        favorited: false,
        reading_progress: 0.0,
        tags: Vec::new(),
        link: output.link,
        // Freshly captured, never yet assigned a folder.
        category_name: None,
        category_icon: None,
    })
}

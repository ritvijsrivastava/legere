use std::path::Path;

use thiserror::Error;

use crate::capture;
use crate::db::DbPool;
use crate::db::queries;
use crate::models::ArticleSummary;
use crate::state::AppState;

#[derive(Debug, Error)]
pub enum DirectLinkError {
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

/// A stored capture, plus whether it was a real one.
pub struct StoredCapture {
    pub article: ArticleSummary,
    /// `Some(reason)` when the capture itself failed and only a link-only
    /// placeholder row was stored (see `capture::capture_local_or_link_only`)
    /// — [`capture_and_store`] returns `Ok` for that case, so a caller that
    /// wants to tell "saved" from "saved a dead link" has to look here.
    /// `None` for a real capture, and also when the link was already in the
    /// library (the existing row is returned untouched).
    // Only the Android share-intent worker reads this (its notification
    // has to tell the two outcomes apart); desktop callers go through
    // `capture_and_store`, which drops it.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub capture_error: Option<String>,
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
    Ok(
        capture_and_store_with_retries(pool, http_client, data_dir, url, 0)
            .await?
            .article,
    )
}

/// [`capture_and_store`], retrying a transient capture failure up to
/// `retries` more times first (see
/// `capture::capture_local_or_link_only_with_retries`) and reporting
/// whether what got stored is a real capture.
pub async fn capture_and_store_with_retries(
    pool: &DbPool,
    http_client: &reqwest::Client,
    data_dir: &Path,
    url: &str,
    retries: u32,
) -> Result<StoredCapture, DirectLinkError> {
    let id = uuid::Uuid::new_v4().to_string();
    // Capture itself never fails this call outright — a dead link still
    // comes back as a link-only `LocalCaptureOutput` (`capture_failed =
    // true`) and gets stored like any other article; see
    // `capture::capture_local_or_link_only`.
    let output =
        capture::capture_local_or_link_only_with_retries(http_client, data_dir, &id, url, retries)
            .await;

    let conn = pool.get()?;
    let inserted = queries::insert_captured_article(&conn, &id, None, "direct", &output, &[])?;

    // `output.link`'s cleaned form was already saved under a different id
    // (the user re-submitted a URL they already have) — the fresh `id`
    // below was never actually inserted, so return the article that's
    // really stored under that link instead of a summary describing a
    // row that doesn't exist.
    if !inserted && let Some(existing) = queries::get_article_summary_by_link(&conn, &output.link)?
    {
        return Ok(StoredCapture {
            article: existing,
            capture_error: None,
        });
    }

    let article = ArticleSummary {
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
    };

    Ok(StoredCapture {
        article,
        capture_error: output.capture_error,
    })
}

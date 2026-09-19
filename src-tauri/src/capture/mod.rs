pub mod archive;
pub mod extract;
pub mod fetch;
pub mod hero_image;
pub mod image_optimize;
pub mod localize;
pub mod rewrite;
pub mod sanitize;
mod ssrf;

use std::path::Path;

use thiserror::Error;

use crate::urlx::{canonicalize, strip_tracking_params};
use fetch::FetchError;
use localize::LocalizeError;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error(transparent)]
    Fetch(#[from] FetchError),
    #[error("failed to localize content images: {0}")]
    Localize(#[from] LocalizeError),
    #[error("failed to rewrite readable content: {0}")]
    Rewrite(#[from] lol_html::errors::RewritingError),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
}

/// Everything needed to insert a freshly captured article into SQLite. Both
/// the RSS-poll path and the direct-link-submit path converge on
/// [`capture_local`] so they share one
/// fetch->extract->sanitize->localize->store pipeline. There is no
/// full-page archive of the original site — offline reading is the
/// readable view above, and the original page is always just a plain
/// link out to the live site (see `link`).
pub struct LocalCaptureOutput {
    pub title: String,
    /// The article's canonical link with tracking query parameters
    /// stripped — a display/storage value, not necessarily byte-identical
    /// to whatever URL was actually fetched. This is what the reader
    /// links out to for "view original".
    pub link: String,
    /// The URL actually fetched, after following redirects — kept for
    /// diagnostics/tests; not persisted to the database. No non-test code
    /// reads this right now (it existed for the archive-server submission
    /// path this struct no longer has), but it's cheap to keep around and
    /// exactly the value a future feature wanting the post-redirect URL
    /// would need.
    #[allow(dead_code)]
    pub final_url: String,
    pub excerpt: String,
    /// Readable-view HTML: sanitized, with image references rewritten to
    /// `legere-content:/<id>/<path>` tokens that resolve into this
    /// article's own `content/<id>/` directory (see
    /// [`rewrite::rewrite_readable_asset_urls`]).
    pub content_html: String,
    /// Always `Some` out of [`capture_local`] — defaults to capture time
    /// when the source page didn't expose a machine-readable publish
    /// date. Still `Option` because test fixtures build this struct
    /// directly without going through that default.
    pub published_at: Option<String>,
    pub read_time_min: i64,
    /// Relative to `data_dir`, e.g. `media/<id>.jpg`.
    pub hero_image_path: Option<String>,
    /// `false` when extraction fell back to naive extraction — the
    /// readable view may be lower quality for such an article.
    pub extraction_confident: bool,
}

/// Fast local capture: fetch -> readability extraction -> sanitize ->
/// localize *only the assets the readable content references* -> write
/// them to this article's own `content/<id>/` directory -> rewrite
/// readable HTML to point into it. `id` is the article's pre-generated
/// uuid, used to name its on-disk files. `data_dir` is
/// `{app_local_data_dir}/legere`; `media/` and `content/` subdirectories
/// are created if missing.
///
/// `client` is used both for fetching the page itself and for every asset
/// fetch downstream — hero image resize, and anything [`localize`] still
/// needs to fetch while localizing the content fragment. Every request
/// this function makes therefore goes through `client`'s own SSRF guard
/// (see `fetch::build_client`).
pub async fn capture_local(
    client: &reqwest::Client,
    data_dir: &Path,
    id: &str,
    url: &str,
) -> Result<LocalCaptureOutput, CaptureError> {
    let page = fetch::fetch_page(client, url).await?;
    let extracted = extract::extract(&page.html, page.final_url.as_str());
    let sanitized_content =
        sanitize::sanitize(&extracted.content_html).map_err(CaptureError::Rewrite)?;

    let localized = localize::localize_content(client, &sanitized_content, &page.final_url).await?;

    let canonical_final_url = canonicalize(&page.final_url);
    let cleaned_link = strip_tracking_params(canonical_final_url.as_url()).to_string();

    let media_dir = data_dir.join("media");
    let content_dir = data_dir.join("content").join(id);
    tokio::fs::create_dir_all(&media_dir).await?;

    let hero_image_path = match &extracted.hero_image_url {
        Some(hero_url) => {
            // Prefer bytes already fetched while localizing the content
            // fragment over a second network round-trip for the same
            // image — a hit only when the hero image also happens to
            // appear inside the readable content itself (the common
            // case), otherwise falls through to a direct fetch, since
            // there's no full-page localization pass left to reuse.
            let already_fetched = rewrite::resolve_reference(hero_url, &page.final_url)
                .and_then(|resolved| localized.url_map.get(&resolved))
                .and_then(|local_path| localized.assets.iter().find(|a| &a.path == local_path));

            let resized = match already_fetched {
                Some(asset) => hero_image::resize_bytes(&asset.bytes),
                None => hero_image::fetch_and_resize(client, hero_url).await,
            };

            match resized {
                Some(bytes) => {
                    let rel_path = format!("media/{id}.jpg");
                    tokio::fs::write(data_dir.join(&rel_path), bytes).await?;
                    Some(rel_path)
                }
                None => None,
            }
        }
        None => None,
    };

    let content_html = rewrite::rewrite_readable_asset_urls(
        &sanitized_content,
        &page.final_url,
        id,
        &localized.url_map,
    )?;

    archive::write_content_files(&content_dir, &localized).await?;

    // Not every page exposes a machine-readable publish date (or
    // `dom_smoothie` fails to find one) — rather than storing `NULL` and
    // pushing the "what date do we show?" question onto every reader of
    // `published_at`, default it to capture time right here, once, so the
    // column is a real "this article's date" for display/sort purposes
    // even when the source page didn't say.
    let published_at = extracted
        .published_at
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    Ok(LocalCaptureOutput {
        title: extracted.title,
        link: cleaned_link,
        final_url: page.final_url.to_string(),
        excerpt: extracted.excerpt,
        content_html,
        published_at: Some(published_at),
        read_time_min: extracted.read_time_min,
        hero_image_path,
        extraction_confident: extracted.extraction_confident,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support;

    /// Exercises the real local-capture pipeline end-to-end against a
    /// local fixture server (no live network): fetch -> extract ->
    /// sanitize -> scoped localize -> content write -> rewrite readable
    /// HTML -> read the content files back. The key assertion this
    /// pipeline split exists to prove: `content/<id>/` contains only the
    /// images the readable fragment actually references (`photo.jpg`) and
    /// *not* page-level assets the readable view never touches
    /// (`style.css`) — confirming `capture_local` really did narrow the
    /// fetch set down to just what's shown, not the whole page.
    #[tokio::test]
    async fn captures_only_the_readable_content_and_its_own_images() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let base_url = test_support::spawn().await;
        let client = test_support::plain_client();

        let url = format!("{base_url}/article.html?utm_source=newsletter&id=7");
        let output = capture_local(&client, data_dir.path(), "test-article", &url)
            .await
            .expect("capture_local should succeed against the fixture server");

        assert!(
            output.extraction_confident,
            "fixture article should be readable"
        );
        assert!(output.title.contains("Quiet Harbor"));
        assert_eq!(
            output.link,
            format!("{base_url}/article.html?id=7"),
            "utm_source should be stripped, id kept"
        );
        assert_eq!(
            output.final_url, url,
            "final_url should be the post-redirect fetched URL"
        );

        // The readable view must reference the photo via a legere-content
        // token, never the fixture server directly.
        assert!(
            output
                .content_html
                .contains("legere-content:/test-article/"),
            "got: {}",
            output.content_html
        );
        assert!(
            !output.content_html.contains(&base_url),
            "readable content_html must not leak the fixture server's URL: {}",
            output.content_html
        );

        let content_dir = data_dir.path().join("content/test-article");
        let local_path = |url_str: &str| -> String {
            let url = url::Url::parse(url_str).unwrap();
            crate::urlx::local_path_for(&crate::urlx::canonicalize(&url))
                .as_str()
                .to_string()
        };

        // `.jpg` is appended on top of the URL-derived path (which
        // already ended in `.jpg`) rather than replacing it —
        // `capture::image_optimize` tags every successfully decoded
        // image with its real format, and `urlx::LocalPath::with_forced_extension`
        // always appends rather than trying to strip an existing one (see
        // its own doc comment for why). The fixture photo is a real,
        // decodable JPEG, so this is deterministic.
        let photo_path = content_dir.join(format!(
            "{}.jpg",
            local_path(&format!("{base_url}/photo.jpg"))
        ));
        let photo_bytes = tokio::fs::read(&photo_path)
            .await
            .expect("photo should be present on disk — the readable content references it");
        assert!(!photo_bytes.is_empty());

        let css_path = content_dir.join(local_path(&format!("{base_url}/style.css")));
        assert!(
            !css_path.exists(),
            "content dir must not contain page-level assets the readable view never references"
        );

        // Capturing the same (cleaned) link again must be a dedup no-op
        // at the database layer, not just avoided by a pre-check.
        let conn = {
            let db_path = data_dir.path().join("dedup-test.db");
            let pool = crate::db::build_pool(&db_path).expect("build pool");
            let mut conn = pool.get().expect("get conn");
            crate::db::schema::migrate(&mut conn).expect("migrate");
            conn
        };
        let inserted_once = crate::db::queries::insert_captured_article(
            &conn,
            "article-one",
            None,
            "direct",
            &output,
            &[],
        )
        .expect("first insert should succeed");
        assert!(inserted_once);

        let second_output = capture_local(&client, data_dir.path(), "test-article-2", &url)
            .await
            .expect("second capture should also succeed");
        let inserted_twice = crate::db::queries::insert_captured_article(
            &conn,
            "article-two",
            None,
            "direct",
            &second_output,
            &[],
        )
        .expect("second insert should not error, just be ignored");
        assert!(
            !inserted_twice,
            "inserting the same cleaned link twice must be a no-op"
        );
    }
}

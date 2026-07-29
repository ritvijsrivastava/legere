pub mod archive;
pub mod extract;
pub mod fetch;
pub mod hero_image;
pub mod localize;
pub mod rewrite;

use std::path::Path;

use thiserror::Error;
use wraith_urlx::{canonicalize, ensure_html_extension, local_path_for, strip_tracking_params};

use fetch::FetchError;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error(transparent)]
    Fetch(#[from] FetchError),
    #[error("failed to sanitize captured HTML: {0}")]
    Sanitize(#[from] wraith_sanitize::SanitizeError),
    #[error("failed to localize page assets: {0}")]
    Localize(#[from] wraith_assets::AssetsError),
    #[error("failed to rewrite readable content: {0}")]
    Rewrite(#[from] lol_html::errors::RewritingError),
    #[error("failed to write ZIM archive: {0}")]
    Zim(#[from] wraith_zim::ZimError),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
}

/// Everything needed to insert a freshly captured article into SQLite. Both
/// the RSS-poll path and the direct-link-submit path converge on
/// [`capture_article`] so they share one
/// fetch->extract->sanitize->localize->archive pipeline.
pub struct CaptureOutput {
    pub title: String,
    /// The article's canonical link with tracking query parameters
    /// stripped — a display/storage value, not necessarily byte-identical
    /// to whatever URL was actually fetched.
    pub link: String,
    pub excerpt: String,
    /// Readable-view HTML: sanitized, with image references rewritten to
    /// `legere-zim:/<id>/<path>` tokens that resolve into this article's
    /// own ZIM archive (see [`rewrite::rewrite_readable_asset_urls`]).
    pub content_html: String,
    pub published_at: Option<String>,
    pub read_time_min: i64,
    /// Relative to `data_dir`, e.g. `media/<id>.jpg`.
    pub hero_image_path: Option<String>,
    /// Relative to `data_dir`, e.g. `archives/<id>.zim`.
    pub zim_path: String,
    /// The archived page's own ZIM entry path (inside the `C` namespace),
    /// for the reader's "Original" view to open directly.
    pub zim_main_path: String,
    /// `false` when extraction fell back to naive extraction — the reader
    /// should default such an article to the archived view.
    pub extraction_confident: bool,
}

/// Captures the article at `url`: fetch -> readability extraction ->
/// sanitize -> localize assets -> ZIM archive -> rewrite readable HTML to
/// point into the archive. `id` is the article's pre-generated uuid, used
/// to name its on-disk files. `data_dir` is `{app_local_data_dir}/legere`;
/// `media/` and `archives/` subdirectories are created if missing.
pub async fn capture_article(
    client: &reqwest::Client,
    data_dir: &Path,
    id: &str,
    url: &str,
) -> Result<CaptureOutput, CaptureError> {
    let page = fetch::fetch_page(client, url).await?;
    let extracted = extract::extract(&page.html, page.final_url.as_str());

    // The archive is a script-free replay of the *whole* page; the
    // readable view is a script-free replay of just the extracted
    // content. Sanitizing both independently keeps that boundary intact
    // regardless of which one a caller ends up rendering.
    let sanitized_page_html = wraith_sanitize::sanitize(&page.html)?;
    let sanitized_content = wraith_sanitize::sanitize(&extracted.content_html)?;

    let localized =
        localize::localize_page(client, &sanitized_page_html, &page.final_url).await?;

    let canonical_final_url = canonicalize(&page.final_url);
    let page_local_path = ensure_html_extension(&local_path_for(&canonical_final_url));
    let cleaned_link = strip_tracking_params(canonical_final_url.as_url()).to_string();

    let media_dir = data_dir.join("media");
    let archives_dir = data_dir.join("archives");
    tokio::fs::create_dir_all(&media_dir).await?;
    tokio::fs::create_dir_all(&archives_dir).await?;

    let hero_image_path = match &extracted.hero_image_url {
        Some(hero_url) => {
            // Prefer bytes already fetched while localizing the page over
            // a second network round-trip for the same image.
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

    let zim_rel_path = format!("archives/{id}.zim");
    let zim_abs_path = data_dir.join(&zim_rel_path);
    let zim_title = extracted.title.clone();
    let zim_main_path = page_local_path.as_str().to_string();
    let zim_main_path_for_write = zim_main_path.clone();
    tokio::task::spawn_blocking(move || {
        archive::write_article_zim(&zim_abs_path, &zim_title, &zim_main_path_for_write, &localized)
    })
    .await
    .expect("zim writer task panicked")?;

    Ok(CaptureOutput {
        title: extracted.title,
        link: cleaned_link,
        excerpt: extracted.excerpt,
        content_html,
        published_at: extracted.published_at,
        read_time_min: extracted.read_time_min,
        hero_image_path,
        zim_path: zim_rel_path,
        zim_main_path,
        extraction_confident: extracted.extraction_confident,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support;

    /// Exercises the real pipeline end-to-end against a local fixture
    /// server (no live network): fetch -> extract -> sanitize -> localize
    /// -> ZIM write -> rewrite readable HTML -> read the ZIM back and
    /// confirm the archive is genuinely self-contained (no remote
    /// references left in either the readable view or the archived page)
    /// and that tracking query parameters were stripped from the stored
    /// link. This is the riskiest part of the MVP, so it gets the most
    /// thorough coverage.
    #[tokio::test]
    async fn captures_a_fixture_page_into_a_self_contained_zim() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let base_url = test_support::spawn().await;
        let client = test_support::plain_client();

        let url = format!("{base_url}/article.html?utm_source=newsletter&id=7");
        let output = capture_article(&client, data_dir.path(), "test-article", &url)
            .await
            .expect("capture_article should succeed against the fixture server");

        assert!(output.extraction_confident, "fixture article should be readable");
        assert!(output.title.contains("Quiet Harbor"));
        assert_eq!(
            output.link,
            format!("{base_url}/article.html?id=7"),
            "utm_source should be stripped, id kept"
        );

        // Expected ZIM entry paths, computed the same way capture/mod.rs
        // itself does — rather than hand-guessing the sanitized layout.
        let zim_local_path = |url_str: &str| -> String {
            let url = url::Url::parse(url_str).unwrap();
            wraith_urlx::local_path_for(&wraith_urlx::canonicalize(&url)).as_str().to_string()
        };
        // Uses the *fetched* URL (query string and all): `local_path_for`
        // deliberately treats distinct query strings as distinct
        // resources, and that's the same `base` both `localize_page` and
        // this main-path computation derive from — see
        // `capture::archive::write_article_zim`'s doc comment.
        let expected_main_path = zim_local_path(&url);
        assert_eq!(output.zim_main_path, expected_main_path);

        // The readable view must reference the photo via a legere-zim
        // token, never the fixture server directly.
        assert!(
            output.content_html.contains("legere-zim:/test-article/"),
            "got: {}",
            output.content_html
        );
        assert!(
            !output.content_html.contains(&base_url),
            "readable content_html must not leak the fixture server's URL: {}",
            output.content_html
        );

        let zim_path = data_dir.path().join(&output.zim_path);
        assert!(zim_path.exists(), "ZIM file should exist on disk");
        let reader = wraith_zim::ZimReader::open(&zim_path).expect("ZIM should be readable back");

        let (main_bytes, main_mime) = reader
            .get_entry_by_full_path(&format!("C{}", output.zim_main_path))
            .expect("reading main page should not error")
            .expect("main page entry should be present");
        assert_eq!(main_mime, "text/html");
        let main_html = String::from_utf8(main_bytes).expect("main page should be UTF-8");
        assert!(
            !main_html.contains(&base_url),
            "archived page must be self-contained, no remote references: {main_html}"
        );

        let css_path = format!("C{}", zim_local_path(&format!("{base_url}/style.css")));
        let (_, css_mime) = reader
            .get_entry_by_full_path(&css_path)
            .expect("reading stylesheet should not error")
            .expect("stylesheet entry should be present");
        assert_eq!(css_mime, "text/css");

        let photo_path = format!("C{}", zim_local_path(&format!("{base_url}/photo.jpg")));
        let (photo_bytes, photo_mime) = reader
            .get_entry_by_full_path(&photo_path)
            .expect("reading photo should not error")
            .expect("photo entry should be present");
        assert_eq!(photo_mime, "image/jpeg");
        assert!(!photo_bytes.is_empty());

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
            "Direct link",
            "direct",
            &output,
        )
        .expect("first insert should succeed");
        assert!(inserted_once);

        let second_output = capture_article(&client, data_dir.path(), "test-article-2", &url)
            .await
            .expect("second capture should also succeed");
        let inserted_twice = crate::db::queries::insert_captured_article(
            &conn,
            "article-two",
            None,
            "Direct link",
            "direct",
            &second_output,
        )
        .expect("second insert should not error, just be ignored");
        assert!(
            !inserted_twice,
            "inserting the same cleaned link twice must be a no-op"
        );
    }
}

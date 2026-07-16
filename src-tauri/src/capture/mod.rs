pub mod archive;
pub mod extract;
pub mod fetch;
pub mod hero_image;

use std::path::Path;

use thiserror::Error;

use fetch::FetchError;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error(transparent)]
    Fetch(#[from] FetchError),
    #[error("failed to sanitize captured HTML: {0}")]
    Sanitize(#[from] scraper1_sanitize::SanitizeError),
    #[error("failed to write ZIM archive: {0}")]
    Zim(#[from] scraper1_zim::ZimError),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
}

/// Everything needed to insert a freshly captured article into SQLite. Both
/// the RSS-poll path and the direct-link-submit path converge on
/// [`capture_article`] so they share one fetch->extract->sanitize->archive
/// pipeline.
pub struct CaptureOutput {
    pub title: String,
    pub link: String,
    pub excerpt: String,
    pub content_html: String,
    pub published_at: Option<String>,
    pub read_time_min: i64,
    /// Relative to `data_dir`, e.g. `media/<id>.jpg`.
    pub hero_image_path: Option<String>,
    /// Relative to `data_dir`, e.g. `archives/<id>.zim`.
    pub zim_path: String,
}

/// Captures the article at `url`: fetch -> readability extraction ->
/// sanitize -> hero-image cache -> ZIM archive. `id` is the article's
/// pre-generated uuid, used to name its on-disk files. `data_dir` is
/// `{app_local_data_dir}/legere`; `media/` and `archives/` subdirectories are
/// created if missing.
pub async fn capture_article(
    client: &reqwest::Client,
    data_dir: &Path,
    id: &str,
    url: &str,
) -> Result<CaptureOutput, CaptureError> {
    let page = fetch::fetch_page(client, url).await?;
    let extracted = extract::extract(&page.html, &page.final_url);
    let sanitized_content = scraper1_sanitize::sanitize(&extracted.content_html)?;

    let media_dir = data_dir.join("media");
    let archives_dir = data_dir.join("archives");
    tokio::fs::create_dir_all(&media_dir).await?;
    tokio::fs::create_dir_all(&archives_dir).await?;

    let hero_image_path = if let Some(hero_url) = &extracted.hero_image_url {
        match hero_image::fetch_and_resize(client, hero_url).await {
            Some(bytes) => {
                let rel_path = format!("media/{id}.jpg");
                tokio::fs::write(data_dir.join(&rel_path), bytes).await?;
                Some(rel_path)
            }
            None => None,
        }
    } else {
        None
    };

    let zim_rel_path = format!("archives/{id}.zim");
    let zim_abs_path = data_dir.join(&zim_rel_path);
    let zim_title = extracted.title.clone();
    let zim_html = page.html.clone();
    tokio::task::spawn_blocking(move || archive::write_article_zim(&zim_abs_path, &zim_title, &zim_html))
        .await
        .expect("zim writer task panicked")?;

    Ok(CaptureOutput {
        title: extracted.title,
        link: page.final_url,
        excerpt: extracted.excerpt,
        content_html: sanitized_content,
        published_at: extracted.published_at,
        read_time_min: extracted.read_time_min,
        hero_image_path,
        zim_path: zim_rel_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exercises the real pipeline end-to-end against a live, stable URL:
    /// fetch -> extract -> sanitize -> ZIM write -> read the ZIM back and
    /// confirm its content matches what was fetched. This is the riskiest
    /// part of the MVP, so it's covered by hitting the network for real
    /// rather than mocking it.
    #[tokio::test]
    async fn captures_a_real_page_and_writes_a_readable_zim() {
        let data_dir = tempfile::tempdir().expect("tempdir");
        let client = fetch::build_client();

        let output = capture_article(&client, data_dir.path(), "test-article", "https://example.com/")
            .await
            .expect("capture_article should succeed against example.com");

        assert!(!output.title.is_empty());
        assert!(!output.content_html.is_empty());
        assert_eq!(output.zim_path, "archives/test-article.zim");

        let zim_path = data_dir.path().join(&output.zim_path);
        assert!(zim_path.exists(), "ZIM file should exist on disk");

        let reader = scraper1_zim::ZimReader::open(&zim_path).expect("ZIM should be readable back");
        let main_page = reader
            .main_page()
            .expect("reading main page should not error")
            .expect("main page should be present");
        let main_page_html = String::from_utf8(main_page).expect("main page should be UTF-8");
        assert!(main_page_html.contains("Example Domain"));
    }
}

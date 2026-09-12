use thiserror::Error;

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("server returned status {0}")]
    Status(reqwest::StatusCode),
}

pub struct FetchedPage {
    /// The final URL after following redirects — used as the base for
    /// resolving every relative reference on the page (both asset
    /// localization and readability extraction) and as the source of the
    /// article's canonical/cleaned link.
    pub final_url: url::Url,
    pub html: String,
}

/// Fetches `url` through the SSRF-guarded client (see [`super::ssrf`]),
/// following redirects and returning the final resolved URL alongside the
/// raw HTML body.
pub async fn fetch_page(client: &reqwest::Client, url: &str) -> Result<FetchedPage, FetchError> {
    let response = client.get(url).send().await?;
    let final_url = response.url().clone();
    let status = response.status();
    if !status.is_success() {
        return Err(FetchError::Status(status));
    }
    let html = response.text().await?;
    Ok(FetchedPage { final_url, html })
}

pub fn build_client() -> reqwest::Client {
    super::ssrf::ssrf_guarded_client_builder()
        .build()
        .expect("SSRF-guarded reqwest client builder should always be constructible")
}

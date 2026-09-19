//! Fetches and localizes every content image the readable view references
//! (`img[src]`, `img[srcset]`/`source[srcset]`, `video[poster]` — the same
//! set [`super::rewrite`] knows how to rewrite), so the reader works fully
//! offline. Deliberately scoped to just the content fragment rather than a
//! whole page: there's no page chrome, unused stylesheets, or script assets
//! to worry about — Readability extraction and [`super::sanitize`] have
//! already narrowed everything down to what's actually shown.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use futures_util::{StreamExt, stream};
use lol_html::{RewriteStrSettings, element};
use sha2::{Digest, Sha256};
use thiserror::Error;
use url::Url;

use crate::urlx::{LocalPath, canonicalize, local_path_for};

use super::image_optimize;
use super::rewrite::{resolve_reference, select_srcset_entry};
use super::ssrf;

/// How many assets are fetched concurrently per article — high enough that
/// a page with a few dozen images localizes quickly, low enough not to
/// look like a burst of unrelated traffic to the origin.
const CONCURRENCY: usize = 6;

/// Caps any single asset's size — protects against an oversized embedded
/// video or unoptimized hero image ballooning the article's storage. An
/// asset over this size is simply left pointing at its original remote
/// URL, the same graceful-degradation path a failed fetch already takes.
const MAX_ASSET_BYTES: u64 = 20 * 1024 * 1024;

/// One retry beyond the first attempt — enough to ride out a transient
/// blip without one slow asset materially delaying a sync loop that may be
/// capturing up to 30 articles in a row.
const MAX_RETRIES: u32 = 1;

const BASE_RETRY_DELAY: Duration = Duration::from_millis(200);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(5);

#[derive(Debug, Error)]
pub enum LocalizeError {
    #[error("failed to discover content asset references: {0}")]
    Discover(#[from] lol_html::errors::RewritingError),
}

/// A single localized content image, ready to be written to disk. Served
/// back out with a MIME type guessed from its extension (see
/// `content_server::guess_content_type`), not the `Content-Type` header
/// the original fetch returned — narrow enough a set of formats
/// (`img`/`source`/`video` references only) that persisting the real
/// header per-asset isn't worth the bookkeeping.
pub struct LocalizedAsset {
    pub path: LocalPath,
    pub bytes: Vec<u8>,
}

pub struct LocalizedContent {
    pub assets: Vec<LocalizedAsset>,
    /// Every discovered asset URL mapped to the [`LocalPath`] it was
    /// localized to. A failed/skipped fetch has no entry — the caller's
    /// rewrite pass ([`super::rewrite::rewrite_readable_asset_urls`])
    /// leaves that reference pointing at its original remote URL. Two URLs
    /// with byte-identical content map to the *same* [`LocalPath`] (see
    /// the content-hash dedup in [`localize_content`]) — `url_map.len()`
    /// can therefore exceed `assets.len()`.
    pub url_map: HashMap<Url, LocalPath>,
}

/// Discovers every `img[src]`/`img,source[srcset]`/`video[poster]`
/// reference in `content_html`, resolved against `base`.
fn discover_references(content_html: &str, base: &Url) -> Result<HashSet<Url>, LocalizeError> {
    // Shared (not `&mut`) across the three handlers below — `element!`
    // closures each need their own capture, so a plain `&mut HashSet`
    // can't be borrowed by more than one of them at once.
    let found = RefCell::new(HashSet::new());
    let collect = |raw: &str| {
        if let Some(url) = resolve_reference(raw, base) {
            found.borrow_mut().insert(url);
        }
    };

    let src_handler = element!("img[src]", |el| {
        if let Some(raw) = el.get_attribute("src") {
            collect(&raw);
        }
        Ok(())
    });
    let srcset_handler = element!("img[srcset], source[srcset]", |el| {
        if let Some(raw) = el.get_attribute("srcset")
            && let Some(chosen) = select_srcset_entry(&raw)
        {
            collect(chosen);
        }
        Ok(())
    });
    let poster_handler = element!("video[poster]", |el| {
        if let Some(raw) = el.get_attribute("poster") {
            collect(&raw);
        }
        Ok(())
    });

    let settings = RewriteStrSettings::new()
        .append_element_content_handler(src_handler)
        .append_element_content_handler(srcset_handler)
        .append_element_content_handler(poster_handler);

    // Discovery never mutates anything, so the rewritten output is
    // discarded — only the handlers' side effect (populating `found`)
    // matters.
    lol_html::rewrite_str(content_html, settings)?;

    Ok(found.into_inner())
}

fn backoff_delay(attempt: u32) -> Duration {
    let scale = 1u64.checked_shl(attempt).unwrap_or(u64::MAX);
    (BASE_RETRY_DELAY.saturating_mul(scale.min(u32::MAX as u64) as u32)).min(MAX_RETRY_DELAY)
}

fn is_retryable(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect() || err.status().is_some_and(|s| s.is_server_error())
}

/// Fetches `url`'s bytes, retrying transient failures with exponential
/// backoff. Returns `None` (never an error) on any failure — a failed
/// fetch degrades to "leave the original remote link", not a capture
/// failure.
async fn fetch_one(client: &reqwest::Client, url: &Url) -> Option<Vec<u8>> {
    // The mandatory half of the SSRF guard for literal-IP URLs — a
    // request whose host is already a literal IP never invokes the
    // client's guarded DNS resolver at all. Checked once: the outcome
    // can't change between retry attempts against the same URL.
    if ssrf::literal_ip_is_blocked(url) {
        tracing::warn!(%url, "refusing to fetch content image: literal IP is blocked");
        return None;
    }

    let mut attempt = 0u32;
    loop {
        match fetch_one_attempt(client, url).await {
            Ok(result) => return result,
            Err(err) if attempt < MAX_RETRIES && is_retryable(&err) => {
                tokio::time::sleep(backoff_delay(attempt)).await;
                attempt += 1;
            }
            Err(err) => {
                tracing::warn!(%url, error = %err, "failed to fetch content image; leaving original remote link");
                return None;
            }
        }
    }
}

/// `Ok(None)` for a deterministic, non-retryable failure (oversized
/// asset); `Err` for a transient `reqwest` failure the caller may retry.
async fn fetch_one_attempt(
    client: &reqwest::Client,
    url: &Url,
) -> Result<Option<Vec<u8>>, reqwest::Error> {
    let response = client.get(url.clone()).send().await?;
    let response = response.error_for_status()?;

    if let Some(len) = response.content_length()
        && len > MAX_ASSET_BYTES
    {
        tracing::warn!(%url, len, "content image exceeds size cap; leaving original remote link");
        return Ok(None);
    }

    let mut buf = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if buf.len() as u64 + chunk.len() as u64 > MAX_ASSET_BYTES {
            tracing::warn!(%url, "content image exceeded size cap mid-stream; leaving original remote link");
            return Ok(None);
        }
        buf.extend_from_slice(&chunk);
    }

    Ok(Some(buf))
}

/// Discovers, then fetches (bounded by [`CONCURRENCY`]), every content
/// image `content_html` — the readable view's own content fragment,
/// already sanitized — references. `base` should be the page's own final
/// URL (after redirects), used to resolve any relative reference.
pub async fn localize_content(
    client: &reqwest::Client,
    content_html: &str,
    base: &Url,
) -> Result<LocalizedContent, LocalizeError> {
    let references = discover_references(content_html, base)?;

    let fetched: Vec<(Url, Vec<u8>)> = stream::iter(references)
        .map(|url| {
            let client = client.clone();
            async move { fetch_one(&client, &url).await.map(|bytes| (url, bytes)) }
        })
        .buffer_unordered(CONCURRENCY)
        .filter_map(|result| async move { result })
        .collect()
        .await;

    // Re-encoding runs on the blocking thread pool, not inline in this
    // async stream: `optimize_content_image`'s resize/re-encode is real
    // CPU work (a Lanczos3 resize on a several-thousand-pixel-wide source
    // image is not free), and this function otherwise only ever awaits
    // I/O. A join failure (which `optimize_content_image` itself has no
    // path to trigger — it never panics) falls back to the original
    // fetched bytes rather than losing the asset entirely.
    let fetched: Vec<(Url, Vec<u8>)> = stream::iter(fetched)
        .map(|(url, bytes)| async move {
            let fallback = bytes.clone();
            let optimized =
                tokio::task::spawn_blocking(move || image_optimize::optimize_content_image(&bytes))
                    .await
                    .unwrap_or(fallback);
            (url, optimized)
        })
        .buffer_unordered(CONCURRENCY)
        .collect()
        .await;

    // Different URLs occasionally resolve to byte-identical content — a
    // CDN that ignores a `srcset` width parameter and serves the same
    // master image for every requested size is a real case this hits (one
    // saved article stored the same photo under eight different URLs
    // before this dedup existed). Content-hash rather than URL-dedup: two
    // URLs with identical bytes share one on-disk file instead of each
    // getting their own copy.
    let mut by_hash: HashMap<[u8; 32], LocalPath> = HashMap::with_capacity(fetched.len());
    let mut url_map = HashMap::with_capacity(fetched.len());
    let mut assets = Vec::with_capacity(fetched.len());
    for (url, bytes) in fetched {
        let hash: [u8; 32] = Sha256::digest(&bytes).into();
        let path = match by_hash.get(&hash) {
            Some(existing) => existing.clone(),
            None => {
                let path = local_path_for(&canonicalize(&url));
                by_hash.insert(hash, path.clone());
                assets.push(LocalizedAsset {
                    path: path.clone(),
                    bytes,
                });
                path
            }
        };
        url_map.insert(url, path);
    }

    Ok(LocalizedContent { assets, url_map })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::routing::get;

    /// Two distinct paths serving byte-identical content — the same shape
    /// as a CDN that ignores a width parameter and returns its master
    /// image regardless of the requested size (see `select_srcset_entry`'s
    /// own doc comment for the real-world case this models).
    async fn spawn_duplicate_content_server() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let base_url = format!("http://localhost:{}", listener.local_addr().unwrap().port());
        let app = Router::new()
            .route("/a.jpg", get(|| async { [0x11u8, 0x22, 0x33, 0x44] }))
            .route("/b.jpg", get(|| async { [0x11u8, 0x22, 0x33, 0x44] }));
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });
        base_url
    }

    #[tokio::test]
    async fn two_urls_with_identical_bytes_share_one_stored_asset() {
        let base_url = spawn_duplicate_content_server().await;
        let base = Url::parse(&base_url).unwrap();
        let client = reqwest::Client::new();
        let html = format!(r#"<img src="{base_url}/a.jpg"><img src="{base_url}/b.jpg">"#);

        let localized = localize_content(&client, &html, &base).await.unwrap();

        assert_eq!(
            localized.assets.len(),
            1,
            "identical bytes from two different URLs must be stored once"
        );
        assert_eq!(localized.url_map.len(), 2, "both URLs still resolve");
        let paths: HashSet<_> = localized.url_map.values().collect();
        assert_eq!(paths.len(), 1, "both URLs must map to the same LocalPath");
    }
}

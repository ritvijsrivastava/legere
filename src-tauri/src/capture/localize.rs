use std::collections::HashMap;

use url::Url;
use wraith_assets::{AssetCache, FetchPolicy, FetchedAsset, LocalizedPage, localize_html};

use super::CaptureError;

/// How many assets `localize_html` fetches concurrently per article — high
/// enough that a page with a few dozen images/fonts localizes quickly,
/// low enough not to look like a burst of unrelated traffic to the origin.
const CONCURRENCY: usize = 6;

/// Caps any single asset's size. A capture runs against arbitrary
/// third-party pages, so this is what actually protects an archive from
/// ballooning on an oversized asset (an embedded video, an unoptimized
/// hero image) — there's no cross-asset total budget in `wraith_assets`,
/// so this per-asset cap is what "cap total asset bytes" comes down to in
/// practice: an asset over this size is simply left pointing at its
/// original remote URL rather than fetched, the same graceful-degradation
/// path a failed fetch already takes.
const MAX_ASSET_BYTES: u64 = 20 * 1024 * 1024;

/// A single retry beyond the first attempt — enough to ride out a
/// transient blip without one slow asset materially delaying a sync loop
/// that may be capturing up to 30 articles in a row.
const MAX_ASSET_RETRIES: u32 = 1;

/// Localizes every asset referenced by `sanitized_page_html` (already
/// script-stripped) for archival: fetches each one over plain HTTP(S) and
/// rewrites the page to reference local copies. `base` should be the
/// page's own final URL (after redirects).
///
/// `network_log` is bytes a real rendering engine already observed while
/// loading the page (see `capture::render`) — consulted ahead of a
/// redundant fetch for any reference this pass discovers, the same hybrid
/// strategy `wraith_core::Archiver` uses with a real browser's own network
/// log. On platforms with no such engine in the loop (the plain-fetch
/// `render` path), this is always empty: every asset reference is
/// resolved via a direct fetch instead.
pub async fn localize_page(
    client: &reqwest::Client,
    sanitized_page_html: &str,
    base: &Url,
    network_log: &HashMap<Url, FetchedAsset>,
) -> Result<LocalizedPage, CaptureError> {
    let policy = FetchPolicy {
        allow_private_network: false,
        max_asset_bytes: MAX_ASSET_BYTES,
        max_retries: MAX_ASSET_RETRIES,
    };
    let cache = AssetCache::new();
    localize_html(
        sanitized_page_html,
        base,
        client,
        CONCURRENCY,
        network_log,
        policy,
        &cache,
    )
    .await
    .map_err(CaptureError::from)
}

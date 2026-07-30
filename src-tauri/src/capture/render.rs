//! Renders a page into HTML plus (when a real rendering engine is
//! available) a network log of every resource that engine actually
//! observed loading.
//!
//! The static, browser-less fetch (`Renderer::Static`) is what every
//! platform used before this module existed, and remains what
//! Windows/macOS use today — a plain HTTP fetch is fully at the mercy of
//! whatever a static parse of the response can find, which is a real gap
//! (a UTF-8 BOM in a fetched stylesheet once broke `lightningcss`'s parse
//! and silently dropped every font reference in the file — a real
//! rendering engine's own text decoder never has this problem). Linux
//! (`render_linux`) and Android (`render_android`) instead drive an
//! off-screen instance of the platform's own WebView/WebKitWebView, since
//! both already ship a full engine — no separate browser process to
//! bundle.
//!
//! [`Renderer`] is which backend to use, chosen *explicitly* by the
//! caller rather than auto-selected from the target platform — a real
//! WebView's requests go through the OS's own network stack, with no
//! `wraith_assets::ssrf_guarded_client_builder()` in the path the way the
//! static fetch has, so switching a call site over to `Renderer::Webview`
//! is a decision to make deliberately per call site, not something that
//! should happen silently because of what platform the binary happens to
//! be running on.

use std::collections::HashMap;

use url::Url;
use wraith_assets::FetchedAsset;

use super::fetch::{self, FetchError};

/// What one platform's render backend hands back to the capture pipeline.
pub struct RenderedPage {
    /// The final URL after following redirects — see
    /// [`fetch::FetchedPage::final_url`] for why this matters.
    pub final_url: Url,
    pub html: String,
    /// Every resource this backend actually observed loading, resolved
    /// URL -> bytes/content-type. Passed straight through to
    /// `localize::localize_page`'s own `network_log` parameter, which
    /// consults it ahead of a redundant fetch. Empty on the plain-fetch
    /// path, since there's no engine there to observe anything beyond the
    /// page response itself.
    pub network_log: HashMap<Url, FetchedAsset>,
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error(transparent)]
    Fetch(#[from] FetchError),
    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[error("webview capture failed: {0}")]
    Webview(String),
}

/// Which backend should render a page — see this module's own docs for
/// why this is an explicit choice at the call site rather than an
/// automatic one keyed off `cfg(target_os)`.
pub enum Renderer<'a> {
    /// Plain HTTP fetch, no rendering engine involved. Every asset fetch
    /// downstream of this (both the page itself and everything
    /// `wraith_assets` localizes) goes through `client`'s own SSRF guard.
    Static(&'a reqwest::Client),
    /// A real, JS-disabled WebView/WebKitWebView, dispatched onto
    /// `AppHandle`'s main thread — see `render_linux`/`render_android`'s
    /// module docs for why JS is off and how each platform's network log
    /// is captured.
    ///
    /// Not yet constructed by any production call site — real WebView
    /// requests bypass `wraith_assets::ssrf_guarded_client_builder()`
    /// entirely (there's no `reqwest::Client` in that path to guard),
    /// which needs its own resolution before an RSS-feed-/user-submitted
    /// URL should ever be handed to this variant. Each backend's own
    /// tests exercise it directly in the meantime.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[allow(dead_code)]
    Webview(&'a tauri::AppHandle),
}

pub async fn render(renderer: Renderer<'_>, url: &str) -> Result<RenderedPage, RenderError> {
    match renderer {
        Renderer::Static(client) => {
            let page = fetch::fetch_page(client, url).await?;
            Ok(RenderedPage {
                final_url: page.final_url,
                html: page.html,
                network_log: HashMap::new(),
            })
        }
        #[cfg(target_os = "linux")]
        Renderer::Webview(app_handle) => super::render_linux::render(app_handle, url).await,
        #[cfg(target_os = "android")]
        Renderer::Webview(app_handle) => super::render_android::render(app_handle, url).await,
    }
}

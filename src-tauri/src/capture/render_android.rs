//! Android capture backend — dispatches to the `page-capture-plugin`
//! Tauri mobile plugin (Kotlin: `PageCapturePlugin`), which drives a
//! real, off-layout `android.webkit.WebView` (Chromium) with JavaScript
//! disabled. See that plugin's own module docs for why this shape
//! differs from `render_linux`'s passive network-log observation:
//! Android's `shouldInterceptRequest` is an interception point, not an
//! after-the-fact observer, so the Kotlin side fetches each resource
//! itself and relays the bytes back.

use std::collections::HashMap;

use base64::Engine;
use page_capture_plugin::PageCaptureExt;
use url::Url;
use wraith_assets::FetchedAsset;

use super::render::{RenderError, RenderedPage};

pub async fn render(app_handle: &tauri::AppHandle, url: &str) -> Result<RenderedPage, RenderError> {
    let result = app_handle
        .page_capture()
        .capture_page(url)
        .await
        .map_err(|e| RenderError::Webview(e.to_string()))?;

    let final_url = Url::parse(&result.final_url)
        .map_err(|e| RenderError::Webview(format!("invalid final_url: {e}")))?;

    let mut network_log: HashMap<Url, FetchedAsset> = HashMap::new();
    for resource in result.resources {
        let Ok(resource_url) = Url::parse(&resource.url) else {
            continue;
        };
        // Kotlin's `android.util.Base64.NO_WRAP` is plain, unpadded-tolerant
        // standard-alphabet base64 — `general_purpose::STANDARD` matches.
        let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&resource.body_base64)
        else {
            continue;
        };
        network_log.insert(
            resource_url,
            FetchedAsset {
                bytes: bytes.into(),
                content_type: resource.content_type,
            },
        );
    }

    let main = network_log
        .get(&final_url)
        .ok_or_else(|| RenderError::Webview("main document was never captured".into()))?;
    let html = String::from_utf8_lossy(&main.bytes).into_owned();

    Ok(RenderedPage {
        final_url,
        html,
        network_log,
    })
}

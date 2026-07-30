//! Tauri mobile plugin: renders a page via a real, off-layout Android
//! `WebView` for capture, mirroring `capture::render_linux` (see that
//! module's docs for the underlying rationale — engine-correct
//! parsing/typography without the fidelity/dependency cost of bundling a
//! separate browser). JavaScript is disabled on the Kotlin side for the
//! same reason: a captured page's own script would otherwise execute for
//! real during capture, including third-party analytics/tracking.
//!
//! Unlike WebKitGTK's `resource-load-started`/`WebResource::data()` (a
//! passive, after-the-fact observer of a request WebView already made),
//! Android's `WebViewClient.shouldInterceptRequest` is fundamentally an
//! interception point: returning `null` lets the request proceed but
//! forfeits the bytes, while supplying a `WebResourceResponse` requires
//! *fetching it yourself* first. `PageCapturePlugin.kt` does exactly
//! that — a plain `HttpURLConnection` per resource, relaying the bytes
//! back into the `WebResourceResponse` — so there is no single
//! passively-observed network log the way there is on Linux.

use tauri::{Manager, Runtime, plugin::TauriPlugin};

#[cfg(mobile)]
use tauri::plugin::PluginHandle;

#[cfg(target_os = "android")]
const PLUGIN_IDENTIFIER: &str = "com.ritvijsrivastava.legere.pagecapture";

mod error;
pub use error::Error;
#[cfg(mobile)]
type Result<T> = std::result::Result<T, Error>;

#[cfg(mobile)]
#[derive(serde::Serialize)]
struct CaptureArgs<'a> {
    url: &'a str,
}

/// One resource the Kotlin side's own `HttpURLConnection` fetched on the
/// WebView's behalf while it rendered the page.
#[derive(Debug, serde::Deserialize)]
pub struct CapturedResource {
    pub url: String,
    pub content_type: Option<String>,
    /// Base64-encoded — JSON has no native byte-string type, and this
    /// crosses the JNI/JSON boundary `run_mobile_plugin` uses.
    pub body_base64: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CaptureResult {
    pub final_url: String,
    pub resources: Vec<CapturedResource>,
}

pub struct PageCapture<R: Runtime> {
    #[cfg(not(mobile))]
    _marker: std::marker::PhantomData<fn() -> R>,
    #[cfg(mobile)]
    handle: PluginHandle<R>,
}

impl<R: Runtime> PageCapture<R> {
    #[cfg(mobile)]
    pub async fn capture_page(&self, url: &str) -> Result<CaptureResult> {
        self.handle
            .run_mobile_plugin_async("capturePage", CaptureArgs { url })
            .await
            .map_err(Error::from)
    }
}

pub trait PageCaptureExt<R: Runtime> {
    fn page_capture(&self) -> &PageCapture<R>;
}

impl<R: Runtime, T: Manager<R>> PageCaptureExt<R> for T {
    fn page_capture(&self) -> &PageCapture<R> {
        self.state::<PageCapture<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("page-capture-plugin")
        .setup(|app, _api| {
            #[cfg(target_os = "android")]
            let handle = _api.register_android_plugin(PLUGIN_IDENTIFIER, "PageCapturePlugin")?;
            app.manage(PageCapture {
                #[cfg(not(mobile))]
                _marker: std::marker::PhantomData::<fn() -> R>,
                #[cfg(mobile)]
                handle,
            });
            Ok(())
        })
        .build()
}

//! Drives a real, detached `webkit2gtk::WebView` — the same engine
//! Tauri's own UI webview uses on Linux — to render a page for capture,
//! so parsing comes from WebKit's own tolerant, spec-following engine
//! rather than a static parse of the raw fetched bytes. (A UTF-8 BOM in a
//! fetched stylesheet once broke `lightningcss`'s static parse entirely,
//! silently dropping every font reference in the file — a real engine's
//! own decoder just never has that problem.)
//!
//! JavaScript is deliberately disabled
//! (`WebViewExt::settings`/`set_enable_javascript(false)`): a captured
//! page's own script would otherwise run for real during capture,
//! including whatever third-party analytics/tracking it embeds —
//! confirmed happening against a real site during development (a
//! `count.js`/`register`/`post-sleep` beacon fired on every load). This
//! means no fidelity gain for JS-rendered content, only for static
//! parsing/typography — which is the actual gap this backend closes.
//! With JS off, the page's own final HTML is never observably different
//! from what it fetched, so the main document's own captured bytes (see
//! `network_log`, keyed by the post-redirect final URL) are used
//! directly rather than reading back `document.documentElement
//! .outerHTML` — which conveniently also isn't available with JS
//! disabled anyway (WebKit refuses `evaluate_javascript` calls in that
//! state).
//!
//! Must run on the thread already pumping WebKitGTK's own main loop — GTK
//! is not safe to drive from two independent main loops in one process,
//! so [`render`] never spins up its own; in production it's dispatched
//! onto Tauri's own main thread via `AppHandle::run_on_main_thread`,
//! which is *already* continuously running that loop for the app's own
//! UI webview. [`start_capture`] only wires up callbacks and returns
//! immediately — whichever loop is already running drives them the rest
//! of the way to completion.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use url::Url;
use webkit2gtk::{LoadEvent, SettingsExt, URIResponseExt, WebResourceExt, WebViewExt};
use wraith_assets::FetchedAsset;

use super::render::{RenderError, RenderedPage};

/// Debounce after the main frame settles (`load-changed` reaches
/// `Finished`) with zero resources still in flight, before treating the
/// page as done. WebKitGTK has no built-in network-idle concept the way
/// CDP does; this can be short because with JS disabled nothing can kick
/// off a *new* request after the ones the initial HTML/CSS parse already
/// discovered.
const QUIET_PERIOD_MS: u64 = 250;
/// Absolute ceiling regardless of network-idle state, so a page that
/// never truly settles (a slow/broken asset host) still returns
/// something instead of hanging a capture forever.
const MAX_WAIT_SECS: u32 = 30;

thread_local! {
    // The one and only strong reference to each in-flight capture's
    // `WebView`, keyed by an opaque id that carries no GObject reference
    // at all. Every closure the view's own signals retain refers only to
    // this id (a plain `Copy` `u64`) to remove its entry when done, never
    // to a strong handle back to the view itself — so there's no
    // reference cycle (view owns its handlers; if those handlers held a
    // strong `WebView` too, directly or indirectly, the view would never
    // be dropped). `start_capture` inserts; `finish` removes.
    static KEEPALIVE: RefCell<HashMap<u64, webkit2gtk::WebView>> = RefCell::new(HashMap::new());
}

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// Renders `url` via a real, JS-disabled `WebView`, dispatched onto
/// `app_handle`'s main thread.
pub async fn render(app_handle: &tauri::AppHandle, url: &str) -> Result<RenderedPage, RenderError> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let url = url.to_string();
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    app_handle
        .run_on_main_thread(move || {
            start_capture(id, url, move |result| {
                let _ = tx.send(result);
            });
        })
        .map_err(|e| RenderError::Webview(e.to_string()))?;
    rx.await
        .map_err(|_| RenderError::Webview("capture task dropped before finishing".into()))?
}

/// The GTK-thread half: wires up a detached `WebView` and its callbacks,
/// then returns immediately — `on_done` fires later, from whatever main
/// loop is already running on this thread, once the page settles (or the
/// absolute timeout fires). Exposed at `pub(super)` so tests can drive it
/// directly against their own dedicated `glib::MainLoop`, without needing
/// a real `tauri::AppHandle`/running app at all.
pub(crate) fn start_capture(
    id: u64,
    url: String,
    on_done: impl FnOnce(Result<RenderedPage, RenderError>) + 'static,
) {
    let requested_url = match Url::parse(&url) {
        Ok(u) => u,
        Err(e) => return on_done(Err(RenderError::Webview(format!("invalid URL: {e}")))),
    };

    let view = webkit2gtk::WebView::new();
    if let Some(settings) = WebViewExt::settings(&view) {
        settings.set_enable_javascript(false);
    }
    KEEPALIVE.with(|k| k.borrow_mut().insert(id, view.clone()));

    let network_log: Rc<RefCell<HashMap<Url, FetchedAsset>>> = Rc::new(RefCell::new(HashMap::new()));
    let pending_resources = Rc::new(RefCell::new(0usize));
    let load_finished = Rc::new(RefCell::new(false));
    let final_url_slot: Rc<RefCell<Option<Url>>> = Rc::new(RefCell::new(None));
    let done: Rc<RefCell<Option<Box<dyn FnOnce(Result<RenderedPage, RenderError>)>>>> =
        Rc::new(RefCell::new(Some(Box::new(on_done))));

    let finish: Rc<dyn Fn()> = Rc::new({
        let network_log = network_log.clone();
        let final_url_slot = final_url_slot.clone();
        let done = done.clone();
        move || {
            let Some(on_done) = done.borrow_mut().take() else {
                return; // already finished once (debounce timer + absolute timeout can both fire)
            };
            KEEPALIVE.with(|k| {
                k.borrow_mut().remove(&id);
            });
            let final_url = final_url_slot
                .borrow()
                .clone()
                .unwrap_or_else(|| requested_url.clone());
            let log = network_log.borrow();
            match log.get(&final_url) {
                Some(main) => {
                    let html = String::from_utf8_lossy(&main.bytes).into_owned();
                    let network_log = log.clone();
                    drop(log);
                    on_done(Ok(RenderedPage {
                        final_url,
                        html,
                        network_log,
                    }));
                }
                None => {
                    drop(log);
                    on_done(Err(RenderError::Webview(
                        "main document was never captured in the network log".into(),
                    )));
                }
            }
        }
    });

    let maybe_finish: Rc<dyn Fn()> = Rc::new({
        let pending_resources = pending_resources.clone();
        let load_finished = load_finished.clone();
        let done = done.clone();
        let finish = finish.clone();
        move || {
            if done.borrow().is_none() {
                return;
            }
            if *load_finished.borrow() && *pending_resources.borrow() == 0 {
                let finish = finish.clone();
                glib::timeout_add_local(std::time::Duration::from_millis(QUIET_PERIOD_MS), move || {
                    finish();
                    glib::ControlFlow::Break
                });
            }
        }
    });

    {
        let pending_resources = pending_resources.clone();
        let network_log = network_log.clone();
        let maybe_finish = maybe_finish.clone();
        view.connect_resource_load_started(move |_view, resource, _request| {
            *pending_resources.borrow_mut() += 1;

            let network_log = network_log.clone();
            let pending_resources_failed = pending_resources.clone();
            let maybe_finish_failed = maybe_finish.clone();
            let pending_resources = pending_resources.clone();
            let maybe_finish = maybe_finish.clone();
            resource.connect_finished(move |resource| {
                let uri = resource.uri();
                let mime = resource
                    .response()
                    .and_then(|r| r.mime_type())
                    .map(|s| s.to_string());
                let network_log = network_log.clone();
                let pending_resources = pending_resources.clone();
                let maybe_finish = maybe_finish.clone();
                // Only http(s) references are kept — mirrors
                // `wraith_assets::resolve_asset_reference`'s own filter,
                // since a `data:`-URI resource (inline SVGs etc.) carries
                // its own bytes and was never something to localize.
                resource.data(gio::Cancellable::NONE, move |result| {
                    if let (Ok(bytes), Some(uri)) = (result, uri.clone())
                        && let Ok(url) = Url::parse(&uri)
                        && matches!(url.scheme(), "http" | "https")
                    {
                        network_log.borrow_mut().insert(
                            url,
                            FetchedAsset {
                                bytes: bytes.into(),
                                content_type: mime.clone(),
                            },
                        );
                    }
                    *pending_resources.borrow_mut() -= 1;
                    maybe_finish();
                });
            });

            // A resource can also fail outright (DNS/connection error)
            // rather than ever reaching `finished` — must still
            // decrement, or one broken asset hangs the capture until the
            // absolute timeout.
            resource.connect_failed(move |_resource, _error| {
                *pending_resources_failed.borrow_mut() -= 1;
                maybe_finish_failed();
            });
        });
    }

    {
        let final_url_slot = final_url_slot.clone();
        let load_finished = load_finished.clone();
        let maybe_finish = maybe_finish.clone();
        view.connect_load_changed(move |view, event| {
            if event == LoadEvent::Finished {
                // Read now, while `view` is a borrowed callback parameter
                // — nothing here holds a strong `WebView` handle, which
                // is what keeps this whole graph cycle-free.
                *final_url_slot.borrow_mut() = view.uri().and_then(|u| Url::parse(&u).ok());
                *load_finished.borrow_mut() = true;
                maybe_finish();
            }
        });
    }

    {
        let done = done.clone();
        view.connect_load_failed(move |_view, _event, failing_uri, error| {
            if let Some(on_done) = done.borrow_mut().take() {
                KEEPALIVE.with(|k| {
                    k.borrow_mut().remove(&id);
                });
                on_done(Err(RenderError::Webview(format!(
                    "failed to load {failing_uri}: {error}"
                ))));
            }
            true // suppress WebKit's own default error-page rendering
        });
    }

    {
        let finish = finish.clone();
        glib::timeout_add_seconds_local(MAX_WAIT_SECS, move || {
            finish();
            glib::ControlFlow::Break
        });
    }

    view.load_uri(&url);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drives [`start_capture`] directly against a real, live site, on
    /// this test thread's own `glib::MainLoop` — deliberately bypassing
    /// `render`'s `AppHandle::run_on_main_thread` dispatch (there's no
    /// real Tauri app/window here to dispatch onto; `tauri::test
    /// ::mock_app`'s `MockRuntime` doesn't run a real GTK loop either, so
    /// it couldn't drive this regardless). This is the actual risky
    /// logic (webkit2gtk resource capture, the keepalive/reference-cycle
    /// handling, JS-disabled font loading) exercised end to end; the
    /// dispatch wrapper itself is one line of well-documented Tauri API
    /// and isn't separately covered here.
    ///
    /// `#[ignore]`d — needs a live network fetch and a real display
    /// (`DISPLAY`/`WAYLAND_DISPLAY`), same reasoning `wraith-browser`'s
    /// own live tests use. Run explicitly with `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn captures_corrode_dev_with_fonts_and_no_js_beacons() {
        gtk::init().expect("gtk::init should succeed with a real display available");
        let main_loop = glib::MainLoop::new(None, false);

        let result: Rc<RefCell<Option<Result<RenderedPage, RenderError>>>> = Rc::new(RefCell::new(None));
        {
            let result = result.clone();
            let main_loop = main_loop.clone();
            start_capture(
                1,
                "https://corrode.dev/blog/hardening-rust/".to_string(),
                move |r| {
                    *result.borrow_mut() = Some(r);
                    main_loop.quit();
                },
            );
        }
        main_loop.run();

        let page = result
            .borrow_mut()
            .take()
            .expect("on_done should have fired")
            .expect("capture should succeed against the live site");

        assert_eq!(page.final_url.as_str(), "https://corrode.dev/blog/hardening-rust/");
        assert!(page.html.contains("Hardening Rust Code For Production"));

        let font_names = ["InterVariable.woff2", "JetBrainsMono-Regular.woff2", "BebasNeue-Bold.woff2"];
        for name in font_names {
            assert!(
                page.network_log.keys().any(|url| url.as_str().ends_with(name)),
                "expected {name} in the network log, got: {:?}",
                page.network_log.keys().collect::<Vec<_>>()
            );
        }

        // The whole reason JS is disabled: a page whose own script fires
        // analytics/tracking beacons on load must never have those show
        // up here.
        assert!(
            !page.network_log.keys().any(|url| url.host_str() == Some("oxitrack.corrode.dev")),
            "JS-driven analytics beacon leaked into the capture despite JS being disabled"
        );
    }

    #[test]
    fn keepalive_is_empty_before_any_capture_starts() {
        assert_eq!(KEEPALIVE.with(|k| k.borrow().len()), 0);
    }
}

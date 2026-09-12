//! Shared test-only fixture HTTP server. Serves the static fixtures under
//! `tests/fixtures/` (an article page, its stylesheet, and an image) plus
//! an RSS feed body templated with this server's own bound port, so
//! `capture` and `sources::rss` tests can exercise the real
//! fetch->extract->sanitize->localize->archive pipeline against a
//! controlled, offline page instead of the live network.
#![cfg(test)]

use std::net::SocketAddr;

use axum::Router;
use axum::http::header::CONTENT_TYPE;
use axum::routing::get;

const ARTICLE_HTML: &str = include_str!("../tests/fixtures/article.html");
const STYLE_CSS: &str = include_str!("../tests/fixtures/style.css");
const PHOTO_JPG: &[u8] = include_bytes!("../tests/fixtures/photo.jpg");

/// Starts the fixture server on an OS-assigned loopback port and returns
/// its base URL, e.g. `http://localhost:54321` — deliberately `localhost`
/// rather than the literal `127.0.0.1` the socket is actually bound to:
/// `capture::localize`'s fetch has its own pre-flight SSRF check (see
/// `capture::ssrf::literal_ip_is_blocked`) that rejects a URL naming a
/// literal loopback/private IP. `localhost` isn't a *literal* IP string,
/// so it sails past that syntactic check and resolves normally via the OS
/// resolver.
///
/// The server task is intentionally never joined — it simply runs for the
/// rest of the test binary's process lifetime, which is fine for a
/// short-lived test run.
///
/// A plain (non-SSRF-guarded) `reqwest::Client` must still be used against
/// this server for the *page* fetch: `capture::ssrf::ssrf_guarded_client_builder`'s
/// custom DNS resolver would otherwise block `localhost` at the
/// connection level regardless of the literal-IP check above.
pub(crate) async fn spawn() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fixture server");
    let addr: SocketAddr = listener.local_addr().expect("local_addr");
    let base_url = format!("http://localhost:{}", addr.port());

    let feed_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Fixture Feed</title>
    <link>{base_url}</link>
    <item>
      <title>Fixture Article</title>
      <link>{base_url}/article.html</link>
      <guid>{base_url}/article.html</guid>
    </item>
  </channel>
</rss>"#
    );

    let app = Router::new()
        .route(
            "/article.html",
            get(|| async { ([(CONTENT_TYPE, "text/html; charset=utf-8")], ARTICLE_HTML) }),
        )
        .route(
            "/style.css",
            get(|| async { ([(CONTENT_TYPE, "text/css")], STYLE_CSS) }),
        )
        .route(
            "/photo.jpg",
            get(|| async { ([(CONTENT_TYPE, "image/jpeg")], PHOTO_JPG) }),
        )
        .route(
            "/feed.xml",
            get(move || {
                let body = feed_xml.clone();
                async move { ([(CONTENT_TYPE, "application/rss+xml")], body) }
            }),
        );

    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("fixture server should not fail");
    });

    base_url
}

/// A plain, unguarded `reqwest::Client` suitable for talking to
/// [`spawn`]'s loopback fixture server.
pub(crate) fn plain_client() -> reqwest::Client {
    reqwest::Client::new()
}

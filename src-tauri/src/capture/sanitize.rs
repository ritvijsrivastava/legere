//! Streaming HTML sanitizer — vendored from the wraith workspace's
//! `wraith-sanitize` crate.
//!
//! Strips every script-execution vector from extracted article content:
//!
//! - `<script>` elements (inline and external);
//! - `on*` event-handler attributes;
//! - `javascript:`/`data:text/html` URIs in `href`/`src`/`action`;
//! - `<link rel="modulepreload">`, `<link rel="preload" as="script">`, and
//!   `<link rel="serviceworker">`;
//! - `<meta http-equiv="refresh">` whose redirect target is itself a
//!   dangerous URL;
//! - `<iframe>` elements, replaced with a placeholder linking to the
//!   original remote URL (recursive embed archival is out of scope);
//! - `<base>` elements, removed entirely (a remote `<base href>` left in
//!   would make [`super::rewrite`]'s locally-rewritten asset references
//!   resolve against the original remote site instead of local files).
//!
//! Deliberately a *denylist*, not an allowlist: everything else — `<img
//! srcset>`, `<video>`/`<source poster>`, tables, `<figure>`, arbitrary
//! `data-*` attributes — passes through untouched, which is what lets
//! [`super::rewrite::rewrite_readable_asset_urls`]'s `img`/`source`/
//! `video` handlers still find anything to rewrite. An allowlist sanitizer
//! (e.g. ammonia's defaults) would silently strip most of that.
//!
//! Implemented as a single streaming pass over the HTML with [`lol_html`] —
//! no full DOM tree is built, keeping memory flat even on large pages.

use lol_html::errors::RewritingError;
use lol_html::html_content::ContentType;
use lol_html::{RewriteStrSettings, element, rewrite_str};

/// URL-bearing attributes checked for dangerous (`javascript:`,
/// `data:text/html`) schemes.
const DANGEROUS_URL_ATTRIBUTES: [&str; 3] = ["href", "src", "action"];

/// Sanitizes `html`, an already-extracted readable-content fragment,
/// stripping every script-execution vector listed at the module level.
pub fn sanitize(html: &str) -> Result<String, RewritingError> {
    let settings = RewriteStrSettings::new()
        .append_element_content_handler(element!("script", |el| {
            el.remove();
            Ok(())
        }))
        .append_element_content_handler(element!("base", |el| {
            el.remove();
            Ok(())
        }))
        .append_element_content_handler(element!("*", |el| {
            // A single pass strips both `on*` handler attributes and any
            // dangerous-scheme URL attribute on every element, avoiding a
            // second full-document traversal.
            let to_remove: Vec<String> = el
                .attributes()
                .iter()
                .filter(|attr| {
                    let name = attr.name();
                    let is_event_handler = name.starts_with("on") && name.len() > 2;
                    let is_dangerous_url_attr = DANGEROUS_URL_ATTRIBUTES.contains(&name.as_str())
                        && is_dangerous_url(&attr.value());
                    is_event_handler || is_dangerous_url_attr
                })
                .map(|attr| attr.name())
                .collect();
            for name in to_remove {
                el.remove_attribute(&name);
            }
            Ok(())
        }))
        .append_element_content_handler(element!("link", |el| {
            let rel = el.get_attribute("rel").unwrap_or_default();
            let as_attr = el.get_attribute("as").unwrap_or_default();

            let is_modulepreload = has_rel_token(&rel, "modulepreload");
            let is_preload_script =
                has_rel_token(&rel, "preload") && as_attr.eq_ignore_ascii_case("script");
            let is_serviceworker = has_rel_token(&rel, "serviceworker");

            if is_modulepreload || is_preload_script || is_serviceworker {
                el.remove();
            }
            Ok(())
        }))
        .append_element_content_handler(element!("meta", |el| {
            let is_refresh = el
                .get_attribute("http-equiv")
                .is_some_and(|v| v.eq_ignore_ascii_case("refresh"));
            if is_refresh
                && let Some(content) = el.get_attribute("content")
                && let Some(target) = extract_refresh_target(&content)
                && is_dangerous_url(&target)
            {
                el.remove();
            }
            Ok(())
        }))
        .append_element_content_handler(element!("iframe", |el| {
            let safe_src = el
                .get_attribute("src")
                .filter(|src| !src.is_empty() && !is_dangerous_url(src));
            let replacement = iframe_placeholder(safe_src.as_deref());
            el.replace(&replacement, ContentType::Html);
            Ok(())
        }));

    let sanitized = rewrite_str(html, settings)?;
    tracing::debug!(
        input_bytes = html.len(),
        output_bytes = sanitized.len(),
        "sanitized captured HTML"
    );
    Ok(sanitized)
}

/// Returns `true` if `rel_value` (a space-separated list of link-relation
/// tokens) contains `token`, compared case-insensitively as the HTML
/// Living Standard requires for keyword attributes.
fn has_rel_token(rel_value: &str, token: &str) -> bool {
    rel_value
        .split_ascii_whitespace()
        .any(|t| t.eq_ignore_ascii_case(token))
}

fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Builds the HTML snippet that replaces a stripped `<iframe>`: a visible
/// placeholder linking to the original remote URL, rather than attempting
/// to recursively archive arbitrary embedded frames.
fn iframe_placeholder(src: Option<&str>) -> String {
    match src {
        Some(src) => {
            let escaped = escape_html(src);
            format!(
                r#"<div class="legere-iframe-placeholder">Embedded content not archived: <a href="{escaped}">{escaped}</a></div>"#
            )
        }
        None => r#"<div class="legere-iframe-placeholder">Embedded content not archived</div>"#
            .to_string(),
    }
}

/// Returns `true` if `raw` is a URL scheme capable of executing script when
/// followed (`javascript:`) or of smuggling a full active HTML document
/// through a data URI (`data:text/html`).
///
/// `raw` is decoded through two normalization steps before comparison,
/// both required to match what a real browser would actually navigate to:
/// HTML entity decoding (`get_attribute` returns raw, still-encoded
/// source text — `&#9;` isn't a tab until decoded), then stripping
/// tabs/newlines and leading/trailing control characters, mirroring the
/// WHATWG URL parser's own scheme-sniff normalization.
fn is_dangerous_url(raw: &str) -> bool {
    let decoded = html_escape::decode_html_entities(raw);
    let trimmed = decoded.trim_matches(|c: char| c.is_control() || c == ' ');
    let cleaned: String = trimmed
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let lower = cleaned.to_ascii_lowercase();
    lower.starts_with("javascript:") || lower.starts_with("data:text/html")
}

fn strip_prefix_ignore_ascii_case<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() >= prefix.len()
        && s.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
    {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

/// Extracts the redirect target from a `<meta http-equiv="refresh"
/// content="...">` value, e.g. `"5; url=https://example.com"` ->
/// `Some("https://example.com")`, or `"5"` (no target, just a delay) ->
/// `None`.
fn extract_refresh_target(content: &str) -> Option<String> {
    let (_delay, rest) = content.split_once(';')?;
    let rest = rest.trim();
    let target = strip_prefix_ignore_ascii_case(rest, "url=")?;
    Some(target.trim().trim_matches(['\'', '"']).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sanitized(html: &str) -> String {
        sanitize(html).expect("sanitize should not fail on well-formed test fixtures")
    }

    #[test]
    fn strips_inline_and_external_scripts() {
        let out = sanitized(
            r#"<html><body><script>alert(1)</script><script src="evil.js"></script><p>hi</p></body></html>"#,
        );
        assert!(!out.contains("<script"));
        assert!(out.contains("<p>hi</p>"));
    }

    #[test]
    fn strips_event_handler_attributes() {
        let out = sanitized(r#"<button onclick="evil()" data-keep="1">Click</button>"#);
        assert!(!out.contains("onclick"));
        assert!(out.contains(r#"data-keep="1""#));
    }

    #[test]
    fn strips_javascript_href_but_keeps_normal_href() {
        let out = sanitized(
            r#"<a href="javascript:alert(1)">bad</a><a href="https://example.com">good</a>"#,
        );
        assert!(!out.contains("javascript:"));
        assert!(out.contains(r#"href="https://example.com""#));
    }

    #[test]
    fn replaces_iframe_with_placeholder_link() {
        let out = sanitized(r#"<iframe src="https://embed.example.com/video"></iframe>"#);
        assert!(!out.contains("<iframe"));
        assert!(out.contains("legere-iframe-placeholder"));
        assert!(out.contains("https://embed.example.com/video"));
    }

    #[test]
    fn strips_base_href() {
        let out = sanitized(r#"<base href="https://example.com/"><img src="a.png">"#);
        assert!(!out.contains("<base"));
        assert!(out.contains(r#"<img src="a.png">"#));
    }

    /// The whole reason this sanitizer is a denylist rather than an
    /// allowlist: content [`super::rewrite`] rewrites must survive
    /// sanitization untouched.
    #[test]
    fn keeps_srcset_video_and_poster_attributes() {
        let out = sanitized(
            r#"<img src="a.jpg" srcset="a.jpg 1x, b.jpg 2x"><video poster="p.jpg"><source src="v.mp4" srcset="v2.mp4 2x"></video>"#,
        );
        assert!(out.contains(r#"srcset="a.jpg 1x, b.jpg 2x""#));
        assert!(out.contains("<video"));
        assert!(out.contains(r#"poster="p.jpg""#));
        assert!(out.contains(r#"srcset="v2.mp4 2x""#));
    }

    #[test]
    fn output_has_zero_script_execution_vectors_for_adversarial_input() {
        let adversarial = r#"
            <script>alert(1)</script>
            <img src=x onerror="alert(2)">
            <a href="java&#9;script:alert(3)">click</a>
            <svg onload="alert(4)"></svg>
            <link rel="modulepreload" href="evil.mjs">
            <meta http-equiv="refresh" content="0;url=javascript:alert(5)">
            <iframe src="javascript:alert(6)"></iframe>
            <form action="javascript:alert(7)"><input></form>
        "#;
        let out = sanitized(adversarial);
        assert!(!out.contains("<script"));
        assert!(!out.to_ascii_lowercase().contains("javascript:"));
        assert!(!out.contains("onerror"));
        assert!(!out.contains("onload"));
        assert!(!out.contains("<iframe"));
        assert!(!out.contains("java&#9;script"));
        assert!(!out.contains("href="));
    }
}

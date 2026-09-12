use std::collections::HashMap;

use lol_html::{RewriteStrSettings, element, errors::RewritingError, rewrite_str};
use url::Url;

use crate::urlx::LocalPath;

/// Rewrites `img`/`source`/`video` asset references in an already-sanitized
/// readable-content fragment to `legere-content:/<article_id>/<local_path>`
/// tokens, using the URL -> [`LocalPath`] resolution
/// [`super::localize::localize_content`] built while fetching and saving
/// this article's own content images.
///
/// `content_html` is a bare content fragment (Readability's output, not a
/// full `<html>` document), so this runs `rewrite_str` directly over it
/// rather than a full-document parse.
///
/// A reference whose resolved URL has no entry in `url_map` (the asset
/// fetch failed, or Readability's DOM transform introduced a URL never
/// discovered on the raw page) is left pointing at its original remote
/// URL — a graceful-degradation fallback rather than a broken local link.
pub fn rewrite_readable_asset_urls(
    content_html: &str,
    base: &Url,
    article_id: &str,
    url_map: &HashMap<Url, LocalPath>,
) -> Result<String, RewritingError> {
    let src_handler = element!("img[src]", |el| {
        if let Some(raw) = el.get_attribute("src")
            && let Some(rewritten) = rewrite_reference(&raw, base, article_id, url_map)
        {
            el.set_attribute("src", &rewritten)?;
        }
        Ok(())
    });

    let srcset_handler = element!("img[srcset], source[srcset]", |el| {
        if let Some(raw) = el.get_attribute("srcset") {
            let rewritten = rewrite_srcset(&raw, base, article_id, url_map);
            el.set_attribute("srcset", &rewritten)?;
        }
        Ok(())
    });

    let poster_handler = element!("video[poster]", |el| {
        if let Some(raw) = el.get_attribute("poster")
            && let Some(rewritten) = rewrite_reference(&raw, base, article_id, url_map)
        {
            el.set_attribute("poster", &rewritten)?;
        }
        Ok(())
    });

    let settings = RewriteStrSettings::new()
        .append_element_content_handler(src_handler)
        .append_element_content_handler(srcset_handler)
        .append_element_content_handler(poster_handler);

    rewrite_str(content_html, settings)
}

/// Resolves a raw HTML attribute value against `base`, returning `None`
/// for values that can never be a localizable asset reference: HTML
/// entities must be decoded before resolution — `get_attribute` returns
/// the raw, still-encoded attribute text — and only `http`/`https`
/// results are ever meaningful lookups into a `url_map` built the same
/// way. Also reused by [`super::localize::discover_references`], so
/// discovery and rewriting can never disagree about what a reference
/// resolves to.
///
/// Exposed at `pub(crate)` so [`crate::capture`]'s pipeline can reuse the
/// exact same resolution when looking up the hero image among
/// already-fetched assets, rather than duplicating it.
pub(crate) fn resolve_reference(raw: &str, base: &Url) -> Option<Url> {
    let decoded = html_escape::decode_html_entities(raw);
    let trimmed = decoded.trim();
    if trimmed.is_empty() {
        return None;
    }
    let resolved = base.join(trimmed).ok()?;
    match resolved.scheme() {
        "http" | "https" => Some(resolved),
        _ => None,
    }
}

fn rewrite_reference(
    raw: &str,
    base: &Url,
    article_id: &str,
    url_map: &HashMap<Url, LocalPath>,
) -> Option<String> {
    let url = resolve_reference(raw, base)?;
    let local = url_map.get(&url)?;
    Some(format!("legere-content:/{article_id}/{}", local.as_str()))
}

/// Splits a `srcset` attribute value into its comma-separated entries,
/// yielding just each entry's URL part (its width/pixel-density
/// descriptor, if any, dropped) — shared between [`rewrite_srcset`]
/// (rewriting) and [`super::localize`] (discovering what to fetch), so the
/// two passes can never disagree about which URLs a `srcset` references.
pub(crate) fn srcset_url_parts(raw: &str) -> impl Iterator<Item = &str> {
    raw.split(',').map(|entry| {
        entry
            .trim()
            .splitn(2, char::is_whitespace)
            .next()
            .unwrap_or("")
    })
}

/// Rewrites every URL in a `srcset` attribute value, preserving each
/// entry's width/pixel-density descriptor. An entry with no localized copy
/// keeps its original URL text unchanged.
fn rewrite_srcset(
    raw: &str,
    base: &Url,
    article_id: &str,
    url_map: &HashMap<Url, LocalPath>,
) -> String {
    raw.split(',')
        .map(|entry| {
            let trimmed = entry.trim();
            let mut parts = trimmed.splitn(2, char::is_whitespace);
            let url_part = parts.next().unwrap_or("");
            let descriptor = parts.next().map(str::trim).unwrap_or("");

            let rewritten_url = rewrite_reference(url_part, base, article_id, url_map)
                .unwrap_or_else(|| url_part.to_string());

            if descriptor.is_empty() {
                rewritten_url
            } else {
                format!("{rewritten_url} {descriptor}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::urlx::{canonicalize, local_path_for};

    fn base() -> Url {
        Url::parse("https://example.com/article").unwrap()
    }

    fn local(url_str: &str) -> LocalPath {
        local_path_for(&canonicalize(&Url::parse(url_str).unwrap()))
    }

    #[test]
    fn rewrites_img_src_to_legere_content_token() {
        let mut url_map = HashMap::new();
        url_map.insert(
            Url::parse("https://example.com/hero.jpg").unwrap(),
            local("https://example.com/hero.jpg"),
        );

        let out = rewrite_readable_asset_urls(
            r#"<img src="hero.jpg">"#,
            &base(),
            "article-1",
            &url_map,
        )
        .unwrap();
        assert_eq!(
            out,
            r#"<img src="legere-content:/article-1/https/example.com/hero.jpg">"#
        );
    }

    #[test]
    fn leaves_unmapped_reference_untouched() {
        let url_map = HashMap::new();
        let out = rewrite_readable_asset_urls(
            r#"<img src="https://example.com/missing.png">"#,
            &base(),
            "article-1",
            &url_map,
        )
        .unwrap();
        assert!(out.contains(r#"src="https://example.com/missing.png""#), "got: {out}");
    }

    #[test]
    fn rewrites_srcset_preserving_descriptors() {
        let mut url_map = HashMap::new();
        url_map.insert(
            Url::parse("https://example.com/a.png").unwrap(),
            local("https://example.com/a.png"),
        );
        url_map.insert(
            Url::parse("https://example.com/b.png").unwrap(),
            local("https://example.com/b.png"),
        );

        let out = rewrite_readable_asset_urls(
            r#"<img srcset="a.png 1x, b.png 2x">"#,
            &base(),
            "article-1",
            &url_map,
        )
        .unwrap();
        assert!(
            out.contains("legere-content:/article-1/https/example.com/a.png 1x"),
            "got: {out}"
        );
        assert!(
            out.contains("legere-content:/article-1/https/example.com/b.png 2x"),
            "got: {out}"
        );
    }

    #[test]
    fn rewrites_video_poster() {
        let mut url_map = HashMap::new();
        url_map.insert(
            Url::parse("https://example.com/poster.jpg").unwrap(),
            local("https://example.com/poster.jpg"),
        );

        let out = rewrite_readable_asset_urls(
            r#"<video poster="poster.jpg"></video>"#,
            &base(),
            "article-1",
            &url_map,
        )
        .unwrap();
        assert!(
            out.contains(r#"poster="legere-content:/article-1/https/example.com/poster.jpg""#),
            "got: {out}"
        );
    }

    #[test]
    fn resolve_reference_decodes_entities_and_filters_schemes() {
        assert_eq!(
            resolve_reference("img.php?a=1&amp;b=2", &base())
                .unwrap()
                .query(),
            Some("a=1&b=2")
        );
        assert!(resolve_reference("data:image/png;base64,abcd", &base()).is_none());
        assert!(resolve_reference("", &base()).is_none());
    }
}

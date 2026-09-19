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

/// The reader always renders content at one fixed column width — there is
/// no responsive layout that benefits from a page's full set of `srcset`
/// breakpoints. `1600` targets a crisp render on a ~800px logical reading
/// column at 2x pixel density; picked once here rather than per-caller so
/// [`select_srcset_entry`]'s two callers (discovery and rewriting) can
/// never disagree about which candidate "the localized one" means.
const SRCSET_TARGET_WIDTH: u32 = 1600;

/// Assumed CSS width behind a pixel-density (`"2x"`) descriptor, which has
/// no width of its own — `srcset` mixes `w` and `x` descriptors in the wild
/// (never both in one list per spec, but sites are not always spec-clean),
/// so both need a comparable "effective width" to rank against each other
/// and against `SRCSET_TARGET_WIDTH`.
const SRCSET_DENSITY_BASE_WIDTH: u32 = 800;

#[derive(Clone, Copy, Debug, PartialEq)]
enum SrcsetDescriptor {
    Width(u32),
    Density(f64),
    /// No descriptor (a bare URL) or one that failed to parse — treated as
    /// an implicit `1x`, same as the spec's own fallback.
    None,
}

fn parse_descriptor(raw: &str) -> SrcsetDescriptor {
    let raw = raw.trim();
    if let Some(w) = raw.strip_suffix('w')
        && let Ok(value) = w.parse::<u32>()
    {
        return SrcsetDescriptor::Width(value);
    }
    if let Some(x) = raw.strip_suffix('x')
        && let Ok(value) = x.parse::<f64>()
    {
        return SrcsetDescriptor::Density(value);
    }
    SrcsetDescriptor::None
}

fn effective_width(descriptor: SrcsetDescriptor) -> u32 {
    match descriptor {
        SrcsetDescriptor::Width(w) => w,
        SrcsetDescriptor::Density(x) => (SRCSET_DENSITY_BASE_WIDTH as f64 * x).round() as u32,
        SrcsetDescriptor::None => SRCSET_DENSITY_BASE_WIDTH,
    }
}

/// Picks the one `srcset` candidate worth fetching: the smallest entry
/// whose effective width still meets [`SRCSET_TARGET_WIDTH`], or — if
/// every entry falls short — the largest one available. Shared between
/// [`super::localize::discover_references`] (what to fetch) and
/// [`rewrite_srcset`] (what the rewritten attribute keeps), so the two
/// passes can never disagree about which single URL "the localized one"
/// is.
///
/// Returns the entry's raw URL text (still HTML-entity-encoded, not yet
/// resolved against a base) — `None` only for an empty/unparseable
/// attribute.
pub(crate) fn select_srcset_entry(raw: &str) -> Option<&str> {
    raw.split(',')
        .filter_map(|entry| {
            let trimmed = entry.trim();
            if trimmed.is_empty() {
                return None;
            }
            let mut parts = trimmed.splitn(2, char::is_whitespace);
            let url_part = parts.next().unwrap_or("");
            if url_part.is_empty() {
                return None;
            }
            let descriptor = parts
                .next()
                .map(parse_descriptor)
                .unwrap_or(SrcsetDescriptor::None);
            Some((url_part, effective_width(descriptor)))
        })
        .min_by_key(|&(_, width)| {
            // Rank 0 (meets the target) always sorts before rank 1
            // (falls short); within a rank, smaller sorts first for rank 0
            // (smallest sufficient candidate) and `u32::MAX - width` makes
            // the *largest* candidate sort first for rank 1 (best
            // available when nothing meets the target).
            if width >= SRCSET_TARGET_WIDTH {
                (0u8, width)
            } else {
                (1u8, u32::MAX - width)
            }
        })
        .map(|(url_part, _)| url_part)
}

/// Rewrites a `srcset` attribute down to a single entry: whichever URL
/// [`select_srcset_entry`] chose to actually fetch and localize
/// (see [`super::localize::discover_references`]), with no descriptor —
/// there is nothing left to describe a choice between once every other
/// candidate has been dropped. An attribute with no localized copy for its
/// chosen entry falls back to that entry's original URL text unchanged,
/// same graceful-degradation path a failed asset fetch already takes.
fn rewrite_srcset(
    raw: &str,
    base: &Url,
    article_id: &str,
    url_map: &HashMap<Url, LocalPath>,
) -> String {
    match select_srcset_entry(raw) {
        Some(url_part) => rewrite_reference(url_part, base, article_id, url_map)
            .unwrap_or_else(|| url_part.to_string()),
        None => raw.to_string(),
    }
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

        let out =
            rewrite_readable_asset_urls(r#"<img src="hero.jpg">"#, &base(), "article-1", &url_map)
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
        assert!(
            out.contains(r#"src="https://example.com/missing.png""#),
            "got: {out}"
        );
    }

    #[test]
    fn rewrites_srcset_to_just_the_selected_candidate() {
        let mut url_map = HashMap::new();
        url_map.insert(
            Url::parse("https://example.com/b.png").unwrap(),
            local("https://example.com/b.png"),
        );

        // "2x" (=1600 effective) meets the 1600px target and is smaller
        // than "3x" (=2400 effective), so it should be the one selected,
        // fetched, and kept — "a.png" (1x = 800) never gets an entry in
        // `url_map` here, proving it was never fetched at all.
        let out = rewrite_readable_asset_urls(
            r#"<img srcset="a.png 1x, b.png 2x, c.png 3x">"#,
            &base(),
            "article-1",
            &url_map,
        )
        .unwrap();
        assert_eq!(
            out,
            r#"<img srcset="legere-content:/article-1/https/example.com/b.png">"#
        );
    }

    #[test]
    fn select_srcset_entry_picks_smallest_that_meets_the_target_width() {
        assert_eq!(
            select_srcset_entry("a.jpg 320w, b.jpg 1600w, c.jpg 3200w"),
            Some("b.jpg")
        );
    }

    #[test]
    fn select_srcset_entry_falls_back_to_the_largest_when_none_meet_the_target() {
        assert_eq!(select_srcset_entry("a.jpg 320w, b.jpg 640w"), Some("b.jpg"));
    }

    #[test]
    fn select_srcset_entry_treats_a_bare_url_as_1x() {
        assert_eq!(select_srcset_entry("a.jpg"), Some("a.jpg"));
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

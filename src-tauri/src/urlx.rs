//! URL canonicalization, tracking-param stripping, and deterministic
//! URL-to-local-path mapping — vendored (not depended on) from the wraith
//! workspace's `wraith-urlx` crate, trimmed to just what legere's capture
//! pipeline actually uses: [`canonicalize`]/[`strip_tracking_params`] for
//! the article's own display link, and [`local_path_for`] for mapping a
//! localized content image's URL to a safe on-disk path under
//! `content/<article_id>/`.

use std::fmt;

use percent_encoding::percent_decode_str;
use url::Url;

/// A [`Url`] normalized to a canonical form used as the dedup key for the
/// article's own link: drops the fragment (never affects what's fetched)
/// and a redundant explicit default port (`:443` on `https://`, `:80` on
/// `http://`). Scheme/host casing don't need handling here — the `url`
/// crate's WHATWG-compliant parser already lowercases both.
///
/// Query strings are preserved byte-for-byte and never reordered: some
/// servers are order-sensitive, so reordering could silently change what a
/// stored link actually resolves to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalUrl(Url);

impl CanonicalUrl {
    pub fn as_url(&self) -> &Url {
        &self.0
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for CanonicalUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

fn default_port_for_scheme(scheme: &str) -> Option<u16> {
    match scheme {
        "http" | "ws" => Some(80),
        "https" | "wss" => Some(443),
        "ftp" => Some(21),
        _ => None,
    }
}

/// Normalizes `url` into its [`CanonicalUrl`] form.
pub fn canonicalize(url: &Url) -> CanonicalUrl {
    let mut url = url.clone();
    url.set_fragment(None);

    if let (Some(explicit_port), Some(default_port)) =
        (url.port(), default_port_for_scheme(url.scheme()))
        && explicit_port == default_port
    {
        let _ = url.set_port(None);
    }

    CanonicalUrl(url)
}

/// Query parameter names known to carry only tracking/analytics
/// information, never anything that changes which resource is requested.
/// Deliberately excludes bare `ref`/`source`: too many sites use those as
/// real routing/selection parameters.
const EXACT_TRACKING_PARAMS: &[&str] = &[
    "fbclid",
    "gclid",
    "gclsrc",
    "dclid",
    "msclkid",
    "wbraid",
    "gbraid",
    "mc_cid",
    "mc_eid",
    "igshid",
    "igsh",
    "si",
    "_hsenc",
    "_hsmi",
    "hsCtaTracking",
    "mkt_tok",
    "s_kwcid",
    "vero_id",
    "oly_anon_id",
    "oly_enc_id",
    "ck_subscriber_id",
];

fn is_tracking_param(name: &str) -> bool {
    name.starts_with("utm_") || EXACT_TRACKING_PARAMS.contains(&name)
}

/// Strips known tracking query parameters from `url`, returning a new
/// [`Url`] with the rest kept in their original relative order. This is a
/// display/storage-time cleanup for the article's saved link — never apply
/// it to a URL a request is actually made against.
pub fn strip_tracking_params(url: &Url) -> Url {
    let Some(query) = url.query() else {
        return url.clone();
    };

    let kept: Vec<(String, String)> = url::form_urlencoded::parse(query.as_bytes())
        .filter(|(name, _)| !is_tracking_param(name))
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect();

    let mut cleaned = url.clone();
    if kept.is_empty() {
        cleaned.set_query(None);
    } else {
        let mut serializer = url::form_urlencoded::Serializer::new(String::new());
        for (name, value) in &kept {
            serializer.append_pair(name, value);
        }
        cleaned.set_query(Some(&serializer.finish()));
    }
    cleaned
}

/// The FNV-1a 64-bit offset basis/prime, as defined by the FNV hash spec.
/// A small, dependency-free, build-stable hash used only to disambiguate
/// local path collisions (never for security purposes) — deliberately not
/// `std::hash::DefaultHasher`, which only guarantees stability within a
/// single process run, not across compilations or repeated runs.
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn short_hex(hash: u64, len: usize) -> String {
    let full = format!("{hash:016x}");
    full[..len.min(full.len())].to_string()
}

/// Maximum length, in bytes, of a single sanitized path segment — stays
/// well under common filesystem limits (typically 255 bytes/component)
/// even after the disambiguation suffix is appended.
const MAX_SEGMENT_LEN: usize = 120;

/// Hex characters of the disambiguation hash embedded when a collision
/// with some other URL becomes possible.
const DISAMBIGUATOR_LEN: usize = 10;

/// A relative, forward-slash-separated, filesystem-safe path derived
/// deterministically from a [`CanonicalUrl`] — the same [`CanonicalUrl`]
/// always maps to the same [`LocalPath`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LocalPath(String);

impl LocalPath {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LocalPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Derives the deterministic [`LocalPath`] for `url`.
///
/// Layout: `<scheme>/<host>[-port<N>]/<sanitized path segments>`, with
/// `index.html` substituted for an empty final segment. Scheme and any
/// non-default port are always embedded (`http://h/x` and `https://h/x`
/// are different resources and must not collapse onto the same local
/// file). A short content-derived disambiguator is appended to the final
/// segment whenever a collision with some other URL becomes possible: the
/// URL carries a query string, or sanitization was lossy for any segment.
pub fn local_path_for(url: &CanonicalUrl) -> LocalPath {
    let raw = url.as_url();

    let mut lossy = false;
    let host = sanitize_chars(raw.host_str().unwrap_or("_unknown-host_"), &mut lossy);
    let port_suffix = raw.port().map(|p| format!("-port{p}")).unwrap_or_default();
    let authority = format!("{}/{host}{port_suffix}", raw.scheme());

    let mut segments: Vec<String> = match raw.path_segments() {
        Some(segs) => segs.map(|seg| sanitize_segment(seg, &mut lossy)).collect(),
        None => Vec::new(),
    };

    match segments.last().map(String::as_str) {
        None | Some("") => {
            segments.pop();
            segments.push("index.html".to_string());
        }
        _ => {}
    }

    if raw.query().is_some() {
        lossy = true;
    }

    if lossy {
        disambiguate_last_segment(&mut segments, url.as_str());
    }

    LocalPath(format!("{authority}/{}", segments.join("/")))
}

fn sanitize_segment(raw: &str, lossy: &mut bool) -> String {
    let decoded = percent_decode_str(raw).decode_utf8_lossy();

    if decoded == "." || decoded == ".." {
        *lossy = true;
        return "_".to_string();
    }

    sanitize_chars(&decoded, lossy)
}

fn sanitize_chars(input: &str, lossy: &mut bool) -> String {
    let mut sanitized = String::with_capacity(input.len());
    for ch in input.chars() {
        if is_filesystem_safe(ch) {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
            *lossy = true;
        }
    }

    let trimmed = sanitized.trim_end_matches(['.', ' ']);
    if trimmed.len() != sanitized.len() {
        *lossy = true;
    }
    let mut sanitized = trimmed.to_string();

    if sanitized.is_empty() && !input.is_empty() {
        sanitized.push('_');
        *lossy = true;
    }

    if sanitized.len() > MAX_SEGMENT_LEN {
        let mut cut = MAX_SEGMENT_LEN;
        while !sanitized.is_char_boundary(cut) {
            cut -= 1;
        }
        sanitized.truncate(cut);
        *lossy = true;
    }

    sanitized
}

/// Characters allowed to pass through unescaped into a local path segment.
fn is_filesystem_safe(ch: char) -> bool {
    !ch.is_control()
        && !matches!(
            ch,
            '/' | '<' | '>' | ':' | '"' | '\\' | '|' | '?' | '*' | '[' | ']'
        )
}

fn disambiguate_last_segment(segments: &mut [String], source_url: &str) {
    let Some(last) = segments.last_mut() else {
        return;
    };
    let suffix = short_hex(fnv1a64(source_url.as_bytes()), DISAMBIGUATOR_LEN);

    match last.rfind('.') {
        Some(dot) if dot > 0 => {
            let ext = last.split_off(dot);
            last.push('.');
            last.push_str(&suffix);
            last.push_str(&ext);
        }
        _ => {
            last.push('.');
            last.push_str(&suffix);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_default_https_port() {
        let url = Url::parse("https://example.com:443/path").unwrap();
        assert_eq!(canonicalize(&url).as_str(), "https://example.com/path");
    }

    #[test]
    fn keeps_non_default_port() {
        let url = Url::parse("https://example.com:8443/path").unwrap();
        assert_eq!(canonicalize(&url).as_str(), "https://example.com:8443/path");
    }

    #[test]
    fn strips_fragment() {
        let url = Url::parse("https://example.com/path#section-2").unwrap();
        assert_eq!(canonicalize(&url).as_str(), "https://example.com/path");
    }

    #[test]
    fn preserves_query_order() {
        let url = Url::parse("https://example.com/path?b=2&a=1").unwrap();
        assert_eq!(canonicalize(&url).as_str(), "https://example.com/path?b=2&a=1");
    }

    #[test]
    fn strips_utm_params() {
        let url = Url::parse("https://example.com/post?utm_source=newsletter&utm_medium=email&id=42").unwrap();
        assert_eq!(strip_tracking_params(&url).as_str(), "https://example.com/post?id=42");
    }

    #[test]
    fn keeps_bare_ref_and_source() {
        let url = Url::parse("https://example.com/post?ref=homepage&source=digest").unwrap();
        assert_eq!(
            strip_tracking_params(&url).as_str(),
            "https://example.com/post?ref=homepage&source=digest"
        );
    }

    #[test]
    fn drops_query_entirely_when_only_tracking_params_present() {
        let url = Url::parse("https://example.com/post?utm_source=x&fbclid=y").unwrap();
        assert_eq!(strip_tracking_params(&url).as_str(), "https://example.com/post");
    }

    fn map(s: &str) -> LocalPath {
        local_path_for(&canonicalize(&Url::parse(s).unwrap()))
    }

    #[test]
    fn simple_path_is_readable() {
        assert_eq!(map("https://example.com/blog/post.html").as_str(), "https/example.com/blog/post.html");
    }

    #[test]
    fn root_maps_to_index() {
        assert_eq!(map("https://example.com/").as_str(), "https/example.com/index.html");
    }

    #[test]
    fn is_deterministic() {
        let a = map("https://example.com/blog/post.html?x=1");
        let b = map("https://example.com/blog/post.html?x=1");
        assert_eq!(a, b);
    }

    #[test]
    fn distinct_queries_produce_distinct_paths() {
        let a = map("https://example.com/blog/post.html?x=1");
        let b = map("https://example.com/blog/post.html?x=2");
        assert_ne!(a, b);
        assert!(a.as_str().ends_with(".html"));
        assert!(b.as_str().ends_with(".html"));
    }

    #[test]
    fn percent_encoded_dot_dot_is_neutralized() {
        let p = map("https://example.com/%2e%2e/%2e%2e/etc/passwd");
        assert!(!p.as_str().contains(".."));
    }

    #[test]
    fn illegal_filesystem_characters_are_replaced() {
        let p = map("https://example.com/weird%3Aname%3F.html");
        assert!(!p.as_str().contains(':'));
        assert!(!p.as_str().contains('?'));
    }

    #[test]
    fn different_schemes_stay_distinct() {
        let http = map("http://example.com/x");
        let https = map("https://example.com/x");
        assert_ne!(http, https);
    }

    #[test]
    fn ipv6_host_has_no_unsafe_characters() {
        let p = map("http://[::1]:8080/x");
        assert!(!p.as_str().contains(':'));
        assert!(!p.as_str().contains('['));
        assert!(!p.as_str().contains(']'));
    }
}

//! `articles.content_html` is stored gzip-compressed (see schema `V14`/
//! `V15`) rather than as plain `TEXT` \u2014 HTML text compresses well (this
//! is prose plus a handful of repeated tag names/attributes, exactly what
//! DEFLATE is good at) and it was, before this, the single largest
//! non-image contributor to a library's on-disk size. Compression/
//! decompression is isolated here so [`super::queries`] (the read/write
//! boundary) and `schema::V14`'s backfill hook share exactly one
//! implementation.
//!
//! Uses `flate2`'s default pure-Rust `miniz_oxide` backend, not a
//! system/`libz` binding \u2014 no new C dependency, no Android NDK
//! cross-compilation risk (the same reasoning `capture::image_optimize`
//! already applied to skip a `libwebp` binding).

use std::io::{Read, Write};

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

/// Compresses `html` for storage. Encoding failure is not a real
/// possibility here (writing to an in-memory `Vec<u8>` never fails), so
/// this panics rather than threading a `Result` through every caller for
/// an error path that cannot actually occur.
pub fn compress_html(html: &str) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder
        .write_all(html.as_bytes())
        .expect("writing to an in-memory Vec<u8> should never fail");
    encoder
        .finish()
        .expect("finishing an in-memory gzip stream should never fail")
}

/// Decompresses bytes written by [`compress_html`]. Panics on malformed
/// input rather than surfacing a `rusqlite::Result` error: every row this
/// reads either came from `compress_html` itself or `V14`'s migration
/// backfill (which used the same function), so corrupt gzip data here
/// would mean the on-disk database itself is already broken \u2014 the same
/// trust boundary every other column in this row already assumes.
pub fn decompress_html(bytes: &[u8]) -> String {
    let mut decoder = GzDecoder::new(bytes);
    let mut out = String::new();
    decoder
        .read_to_string(&mut out)
        .expect("stored content_html must be valid gzip-compressed UTF-8");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_html_through_compress_and_decompress() {
        let html = "<p>Hello, world! ".repeat(200) + "</p>";
        let compressed = compress_html(&html);
        assert!(
            compressed.len() < html.len(),
            "repetitive HTML should compress smaller: {} vs {}",
            compressed.len(),
            html.len()
        );
        assert_eq!(decompress_html(&compressed), html);
    }

    #[test]
    fn round_trips_empty_and_unicode_content() {
        assert_eq!(decompress_html(&compress_html("")), "");
        let unicode = "caf\u{e9} \u{2014} \u{65e5}\u{672c}\u{8a9e} \u{1f4da}";
        assert_eq!(decompress_html(&compress_html(unicode)), unicode);
    }
}

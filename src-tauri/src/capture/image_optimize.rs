//! Re-encodes fetched content images to shrink their on-disk footprint
//! before [`super::archive::write_content_files`] writes them under
//! `content/<id>/`: downscales anything wider than the reader's own
//! column can ever show, and re-compresses opaque raster images as JPEG
//! at a quality that's visually close to lossless for reading-pane
//! display but meaningfully smaller than most source CDNs' own delivery
//! quality (which routinely serve quality-90+ JPEGs or uncompressed PNGs
//! sized for a full page, not a ~800px reading column).
//!
//! Deliberately *not* re-encoding to WebP: the `image` crate's own WebP
//! encoder (`image-webp`, the pure-Rust backend behind the `webp`
//! feature) only supports lossless encoding, which is reliably smaller
//! than PNG but not than a well-compressed JPEG — using it here could
//! grow photographic content instead of shrinking it. True lossy WebP
//! needs `libwebp` C bindings, a cross-compilation risk for the Android
//! NDK target this project also has to build for; not worth it against
//! JPEG re-encoding already available with zero new dependencies.
//!
//! GIFs (`image`'s `gif` feature is enabled) are flattened to their first
//! frame and re-encoded the same way as any other opaque/transparent
//! source — not kept as animated. A meaningful real-world case, not a
//! hypothetical one: some photo CDNs serve full photographs in a GIF
//! container (8-bit palette, wildly inefficient for photographic content)
//! with no `.gif` extension in the URL at all. Re-encoding these as JPEG
//! rather than re-animating them trades animation for a large size win;
//! acceptable here since Legere's readable view has never played back
//! animated content anyway (the sanitizer already strips `<script>`, and
//! nothing in the reader relies on GIF animation as a feature).
//!
//! Anything else this can't decode (SVG, unrecognized formats) is
//! returned unchanged: this function never fails a capture, it only ever
//! narrows what gets stored, and never returns anything larger than what
//! was fetched.

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat};

/// Matches `capture::rewrite::SRCSET_TARGET_WIDTH` \u2014 the reader never
/// needs a content image wider than the same reading-column target
/// srcset-candidate selection already picks toward.
const MAX_WIDTH: u32 = 1600;

/// Visually close to source quality for in-app reading-pane display;
/// well below what most CDNs deliver by default (often 90+).
const JPEG_QUALITY: u8 = 80;

/// Re-encodes `bytes` if doing so is a net size win; otherwise returns the
/// original bytes untouched. Never errors: any decode/encode failure just
/// means "nothing to optimize here", the same graceful-degradation shape
/// the rest of the localize pipeline already uses for fetch failures.
pub fn optimize_content_image(bytes: &[u8]) -> Vec<u8> {
    let Ok(img) = image::load_from_memory(bytes) else {
        return bytes.to_vec();
    };

    let resized = resize_if_needed(img);

    let encoded = if has_transparency(&resized) {
        encode_png(&resized)
    } else {
        encode_jpeg(&resized)
    };

    match encoded {
        Some(optimized) if optimized.len() < bytes.len() => optimized,
        _ => bytes.to_vec(),
    }
}

fn resize_if_needed(img: DynamicImage) -> DynamicImage {
    if img.width() <= MAX_WIDTH {
        return img;
    }
    let ratio = MAX_WIDTH as f64 / img.width() as f64;
    let new_height = ((img.height() as f64 * ratio).round() as u32).max(1);
    img.resize(MAX_WIDTH, new_height, FilterType::Lanczos3)
}

/// True only when the image actually *uses* transparency (some pixel with
/// alpha < 255) \u2014 an image with an alpha channel that's fully opaque
/// anyway (common: many PNGs are RGBA but never actually transparent) is
/// still safe, and better, to flatten to JPEG rather than kept as PNG.
fn has_transparency(img: &DynamicImage) -> bool {
    if !img.color().has_alpha() {
        return false;
    }
    img.to_rgba8().pixels().any(|p| p[3] < 255)
}

fn encode_jpeg(img: &DynamicImage) -> Option<Vec<u8>> {
    let mut out = Cursor::new(Vec::new());
    let mut encoder = JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY);
    encoder.encode_image(&img.to_rgb8()).ok()?;
    Some(out.into_inner())
}

fn encode_png(img: &DynamicImage) -> Option<Vec<u8>> {
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    fn opaque_photo_like_png(width: u32, height: u32) -> Vec<u8> {
        // A gradient, not a flat fill: flat-color PNGs compress so well
        // under PNG's own filters that a JPEG re-encode wouldn't
        // necessarily win, which would defeat the point of this fixture.
        let buf = ImageBuffer::from_fn(width, height, |x, y| {
            Rgba([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255])
        });
        let mut out = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(buf)
            .write_to(&mut out, ImageFormat::Png)
            .unwrap();
        out.into_inner()
    }

    fn transparent_png(width: u32, height: u32) -> Vec<u8> {
        let buf = ImageBuffer::from_fn(width, height, |x, _y| {
            Rgba([255, 0, 0, if x < width / 2 { 0 } else { 255 }])
        });
        let mut out = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(buf)
            .write_to(&mut out, ImageFormat::Png)
            .unwrap();
        out.into_inner()
    }

    #[test]
    fn shrinks_a_wide_opaque_image_and_reencodes_as_jpeg() {
        let original = opaque_photo_like_png(2400, 1600);
        let optimized = optimize_content_image(&original);

        assert!(
            optimized.len() < original.len(),
            "optimized ({}) should be smaller than original ({})",
            optimized.len(),
            original.len()
        );
        let decoded = image::load_from_memory(&optimized).unwrap();
        assert_eq!(decoded.width(), MAX_WIDTH, "should be capped to MAX_WIDTH");
        assert_eq!(
            image::guess_format(&optimized).unwrap(),
            ImageFormat::Jpeg,
            "opaque images should be re-encoded as JPEG"
        );
    }

    #[test]
    fn keeps_a_transparent_image_as_png_not_jpeg() {
        let original = transparent_png(100, 100);
        let optimized = optimize_content_image(&original);

        assert_eq!(
            image::guess_format(&optimized).unwrap(),
            ImageFormat::Png,
            "transparent images must never be flattened to JPEG"
        );
    }

    #[test]
    fn flattens_a_gif_to_a_smaller_static_jpeg() {
        // A photo-like gradient, encoded through GIF's 256-color palette
        // — mirrors the real case this exists for: a CDN serving
        // photographs in a GIF container. GIF's own LZW+palette encoding
        // is a poor fit for a smooth gradient, so this should shrink hard
        // once re-encoded as JPEG.
        let buf = ImageBuffer::from_fn(800, 600, |x, y| {
            Rgba([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255])
        });
        let mut original = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(buf)
            .write_to(&mut original, ImageFormat::Gif)
            .unwrap();
        let original = original.into_inner();

        let optimized = optimize_content_image(&original);

        assert_eq!(
            image::guess_format(&optimized).unwrap(),
            ImageFormat::Jpeg,
            "a decodable GIF should be flattened to a static JPEG"
        );
        assert!(
            optimized.len() < original.len(),
            "optimized ({}) should be smaller than the GIF original ({})",
            optimized.len(),
            original.len()
        );
    }

    #[test]
    fn leaves_undecodable_bytes_completely_unchanged() {
        let garbage = b"not an image, just some bytes".to_vec();
        assert_eq!(optimize_content_image(&garbage), garbage);
    }

    #[test]
    fn never_returns_something_larger_than_the_original() {
        // A tiny, already-optimal image: re-encoding it at a lossy
        // quality could theoretically add JPEG overhead that isn't worth
        // it at this size. The size-comparison fallback in
        // `optimize_content_image` must catch this rather than trust the
        // re-encode blindly.
        let original = opaque_photo_like_png(4, 4);
        let optimized = optimize_content_image(&original);
        assert!(optimized.len() <= original.len());
    }
}

use std::io::Cursor;

use image::ImageFormat;
use image::imageops::FilterType;

const MAX_WIDTH: u32 = 1200;

/// Downloads the image at `url` and re-encodes it as a bounded-width JPEG,
/// normalizing whatever format the source served and keeping stored hero
/// images small. Returns `None` on any fetch/decode failure — a missing hero
/// image is not a capture failure.
pub async fn fetch_and_resize(client: &reqwest::Client, url: &str) -> Option<Vec<u8>> {
    let bytes = client.get(url).send().await.ok()?.bytes().await.ok()?;
    let img = image::load_from_memory(&bytes).ok()?;

    let resized = if img.width() > MAX_WIDTH {
        let ratio = MAX_WIDTH as f64 / img.width() as f64;
        let new_height = (img.height() as f64 * ratio).round() as u32;
        img.resize(MAX_WIDTH, new_height, FilterType::Lanczos3)
    } else {
        img
    };

    let mut out = Cursor::new(Vec::new());
    resized
        .to_rgb8()
        .write_to(&mut out, ImageFormat::Jpeg)
        .ok()?;
    Some(out.into_inner())
}

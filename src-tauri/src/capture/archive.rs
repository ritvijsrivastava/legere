use std::path::Path;

use super::localize::LocalizedContent;
use crate::db::compression::compress_bytes;

/// Writes every localized content image to `content_dir` (`{data_dir}/
/// content/{article_id}`), one plain file per asset at its own
/// [`crate::urlx::LocalPath`] — e.g. `content_dir/https/example.com/
/// hero.jpg`. This is the small, persistent store behind
/// `legere-content:/<id>/<path>` tokens in an article's readable
/// `content_html` (see [`super::rewrite::rewrite_readable_asset_urls`]):
/// never evicted, so those images stay available for as long as the
/// article itself does.
///
/// `content_dir` is removed and recreated first — a recapture (see
/// `commands::articles::recapture_article`) writes into the same
/// directory a second time, and an asset referenced by the *old* capture
/// but not the new one must not linger. This is what a single-file ZIM
/// archive got for free by being overwritten wholesale; plain files need
/// it done explicitly.
pub async fn write_content_files(
    content_dir: &Path,
    localized: &LocalizedContent,
) -> std::io::Result<()> {
    let _ = tokio::fs::remove_dir_all(content_dir).await;
    for asset in &localized.assets {
        let path = content_dir.join(asset.path.as_str());
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let bytes = if is_svg(asset.path.as_str()) {
            compress_bytes(&asset.bytes)
        } else {
            asset.bytes.clone()
        };
        tokio::fs::write(&path, &bytes).await?;
    }
    Ok(())
}

/// SVG assets are gzip-compressed before being written — unlike raster
/// formats (already re-encoded/downscaled by `image_optimize`), an SVG is
/// passed through untouched because it can't be decoded as a raster image
/// at all, but as XML text it compresses just as well as `content_html`
/// does (some real-world infographic SVGs run several MB uncompressed).
/// `content_server::serve` reverses this on the way out; the `.svg`
/// filename/extension and its `image/svg+xml` MIME type are unaffected,
/// only the bytes on disk change.
fn is_svg(path: &str) -> bool {
    path.rsplit('.')
        .next()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use url::Url;

    use super::*;
    use crate::capture::localize::{LocalizedAsset, LocalizedContent};
    use crate::db::compression::decompress_bytes;
    use crate::urlx::{canonicalize, local_path_for};

    fn local_path(url: &str) -> crate::urlx::LocalPath {
        local_path_for(&canonicalize(&Url::parse(url).unwrap()))
    }

    #[tokio::test]
    async fn writes_an_svg_asset_gzip_compressed_on_disk_but_decompresses_back_to_the_original() {
        let dir = tempfile::tempdir().unwrap();
        let svg = b"<svg xmlns='http://www.w3.org/2000/svg'><circle r='1'/></svg>".to_vec();
        let path = local_path("https://example.com/icon.svg");

        let localized = LocalizedContent {
            assets: vec![LocalizedAsset {
                path: path.clone(),
                bytes: svg.clone(),
            }],
            url_map: HashMap::new(),
        };
        write_content_files(dir.path(), &localized).await.unwrap();

        let on_disk = tokio::fs::read(dir.path().join(path.as_str()))
            .await
            .unwrap();
        assert_ne!(
            on_disk, svg,
            "an SVG asset should be stored gzip-compressed, not as plain bytes"
        );
        assert_eq!(decompress_bytes(&on_disk).unwrap(), svg);
    }

    #[tokio::test]
    async fn writes_a_non_svg_asset_as_plain_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let jpg = vec![0xFFu8, 0xD8, 0xFF, 0xAA, 0xBB];
        let path = local_path("https://example.com/photo.jpg");

        let localized = LocalizedContent {
            assets: vec![LocalizedAsset {
                path: path.clone(),
                bytes: jpg.clone(),
            }],
            url_map: HashMap::new(),
        };
        write_content_files(dir.path(), &localized).await.unwrap();

        let on_disk = tokio::fs::read(dir.path().join(path.as_str()))
            .await
            .unwrap();
        assert_eq!(on_disk, jpg);
    }
}

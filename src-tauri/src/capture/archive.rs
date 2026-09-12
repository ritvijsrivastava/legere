use std::path::Path;

use super::localize::LocalizedContent;

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
        tokio::fs::write(&path, &asset.bytes).await?;
    }
    Ok(())
}

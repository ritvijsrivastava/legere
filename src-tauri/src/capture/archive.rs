use std::path::Path;

use wraith_assets::LocalizedPage;
use wraith_zim::{WriteReport, ZimError, ZimWriter, today_iso8601};

/// Archives only the images the readable view actually references — no
/// page HTML, no `W/mainPage` redirect, since nothing ever asks a content
/// zim for "the main page." This is the small, persistent store behind
/// `legere-zim:/<id>/<path>` tokens in an article's readable `content_html`
/// (see [`super::rewrite::rewrite_readable_asset_urls`]): unlike the full
/// archive, it's never evicted, so those images stay available regardless
/// of whether the full archive is currently cached locally.
pub fn write_content_zim(
    output: &Path,
    article_id: &str,
    localized: &LocalizedPage,
) -> Result<WriteReport, ZimError> {
    let mut writer = ZimWriter::new()
        .name(article_id)
        .title(article_id)
        .creator("Legere")
        .publisher("Legere")
        .date(today_iso8601())
        .description(article_id)
        .language("eng");

    for asset in &localized.assets {
        let mimetype = asset
            .content_type
            .as_deref()
            .unwrap_or("application/octet-stream");
        writer = writer.add_content(asset.path.as_str(), mimetype, "", asset.bytes.clone());
    }

    writer.write(output)
}

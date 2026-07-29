use std::path::Path;

use wraith_assets::LocalizedPage;
use wraith_zim::{WriteReport, ZimError, ZimWriter, today_iso8601};

/// Archives one fully localized page — its HTML plus every fetched asset —
/// as a genuinely self-contained ZIM: nothing inside it points outside the
/// archive, so opening it later with no network needs nothing else.
///
/// `main_page_path` is the ZIM entry path (inside the `C` namespace) the
/// page's HTML is stored under and the `W/mainPage` redirect points at.
/// It must be computed by the caller from the same final URL that was
/// passed as `localize_page`'s `base` (via `wraith_urlx::canonicalize` ->
/// `local_path_for` -> `ensure_html_extension`) so that `localized.html`'s
/// already-computed relative asset references resolve correctly from
/// wherever the page actually ends up stored — `ensure_html_extension`
/// only ever changes a path's final segment, never its directory portion,
/// so this stays consistent with what `wraith_assets::localize_html`
/// resolved internally even though the two paths can differ by extension.
pub fn write_article_zim(
    output: &Path,
    title: &str,
    main_page_path: &str,
    localized: &LocalizedPage,
) -> Result<WriteReport, ZimError> {
    let mut writer = ZimWriter::new()
        .add_content(
            main_page_path,
            "text/html",
            title,
            localized.html.as_bytes().to_vec(),
        )
        .main_page(main_page_path)
        .name(title)
        .title(title)
        .creator("Legere")
        .publisher("Legere")
        .date(today_iso8601())
        .description(title)
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

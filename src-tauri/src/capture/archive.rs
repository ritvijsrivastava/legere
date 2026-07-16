use std::path::Path;

use scraper1_zim::{WriteReport, ZimError, ZimWriter, today_iso8601};

/// Archives the raw (pre-extraction) HTML for one article as a single-page
/// ZIM file. MVP scope deliberately omits recursive asset localization (see
/// the architecture plan) — this is byte-exact archival of the document
/// itself, not a fully self-contained replay of the original page.
pub fn write_article_zim(
    output: &Path,
    title: &str,
    raw_html: &str,
) -> Result<WriteReport, ZimError> {
    ZimWriter::new()
        .add_content("index.html", "text/html", title, raw_html.as_bytes().to_vec())
        .main_page("index.html")
        .name(title)
        .title(title)
        .creator("Legere")
        .publisher("Legere")
        .date(today_iso8601())
        .description(title)
        .language("eng")
        .write(output)
}

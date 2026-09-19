//! CSV export/import of RSS sources (the Sources page's "Export sources"
//! / "Import sources") — a fixed, app-native column shape, independent of
//! `sources::raindrop_import` (bookmarks, not feeds) and
//! `sources::article_csv` (the article-library export/import). `feed_url`
//! is the only column import reads back; the rest of the export exists
//! for a human reading the file, or for context when sharing it.

use serde::{Deserialize, Serialize};

use crate::models::Source;

#[derive(Debug, Serialize)]
struct SourceExportRow<'a> {
    name: &'a str,
    feed_url: &'a str,
    status: &'a str,
    article_count: i64,
    last_synced_at: &'a str,
    created_at: &'a str,
}

/// Builds the CSV bytes for `sources` (see `db::queries::list_sources`).
/// A source with no `feed_url` (shouldn't currently exist — sources are
/// RSS-only since schema V2 — but not assumed here) exports with an empty
/// cell rather than panicking.
pub fn build_csv(sources: &[Source]) -> Result<String, csv::Error> {
    let mut writer = csv::Writer::from_writer(vec![]);
    for source in sources {
        writer.serialize(SourceExportRow {
            name: &source.name,
            feed_url: source.feed_url.as_deref().unwrap_or(""),
            status: &source.status,
            article_count: source.article_count,
            last_synced_at: source.last_synced_at.as_deref().unwrap_or(""),
            created_at: &source.created_at,
        })?;
    }
    let bytes = writer.into_inner().map_err(|e| e.into_error())?;
    Ok(String::from_utf8(bytes).expect("csv writer only ever emits valid utf8 from utf8 input"))
}

#[derive(Debug, Deserialize)]
struct SourceImportRow {
    #[serde(default)]
    #[allow(dead_code)]
    name: String,
    feed_url: String,
}

/// Cheaply checks that `csv_bytes` parses as this module's own shape at
/// all (in particular, that a `feed_url` column exists).
pub fn validate_csv(csv_bytes: &[u8]) -> Result<(), csv::Error> {
    let mut reader = csv::Reader::from_reader(csv_bytes);
    for row in reader.deserialize::<SourceImportRow>() {
        row?;
    }
    Ok(())
}

/// Parses `csv_bytes` and returns every distinct, non-empty `feed_url`
/// cell it contains, in file order — the only column re-added on import
/// (see module docs). A source's display name is always re-derived from
/// its feed on the next sync, same as adding it by hand, so the export's
/// `name` column is read-only/informational and never consulted here.
pub fn parse_feed_urls(csv_bytes: &[u8]) -> Result<Vec<String>, csv::Error> {
    let mut reader = csv::Reader::from_reader(csv_bytes);
    let rows: Vec<SourceImportRow> = reader.deserialize().collect::<Result<Vec<_>, _>>()?;
    Ok(rows
        .into_iter()
        .map(|r| r.feed_url.trim().to_string())
        .filter(|u| !u.is_empty())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(feed_url: &str) -> Source {
        Source {
            id: "id".to_string(),
            name: "Example".to_string(),
            source_type: "rss".to_string(),
            feed_url: Some(feed_url.to_string()),
            status: "active".to_string(),
            last_error: None,
            article_count: 3,
            last_synced_at: Some("2024-01-01T00:00:00Z".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn build_csv_round_trips_feed_urls_through_parse_feed_urls() {
        let sources = vec![source("https://example.com/feed.xml")];
        let csv = build_csv(&sources).expect("builds");
        assert!(csv.contains("name,feed_url,status,article_count,last_synced_at,created_at"));
        let parsed = parse_feed_urls(csv.as_bytes()).expect("parses back");
        assert_eq!(parsed, vec!["https://example.com/feed.xml".to_string()]);
    }

    #[test]
    fn parse_feed_urls_drops_blank_cells() {
        let csv = "name,feed_url\nOne,https://a.example/feed.xml\nTwo,\n";
        let parsed = parse_feed_urls(csv.as_bytes()).expect("parses");
        assert_eq!(parsed, vec!["https://a.example/feed.xml".to_string()]);
    }
}

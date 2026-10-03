//! Full-fidelity row representations of every synced entity type
//! (articles/categories/sources — `settings` stays local, never synced;
//! see ARCHITECTURE.md's Sync section) plus the read/write functions
//! cross-device sync uses to diff its local DB against the manifest and
//! to pull/push/apply rows.
//!
//! Deliberately separate from `db::queries` (which serves the app's own
//! UI-facing reads): these shapes are sync's wire format, not the
//! display model. `models::ArticleSummary`/`ArticleDetail` omit
//! `updated_at`, and `fetched_at`/`created_at` as a `conflict_key`
//! tiebreaker, and several other columns the UI never needs but sync
//! does.

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use super::compression::{compress_html, decompress_html};
use super::queries::{normalize_tags, parse_tags, tags_to_json};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncArticleRow {
    pub id: String,
    pub source_id: Option<String>,
    pub source_type: String,
    pub title: String,
    pub link: String,
    pub excerpt: String,
    /// Plain (decompressed) HTML. The per-entity blob this row is
    /// serialized into is itself gzip-compressed as a whole (mirroring
    /// `manifest::Manifest::to_gz_bytes`), so compressing it a second
    /// time here would just waste CPU re-deflating already-compact text
    /// without shrinking the transferred bytes.
    pub content_html: String,
    pub hero_image_path: Option<String>,
    pub published_at: Option<String>,
    pub fetched_at: String,
    pub read_time_min: i64,
    pub reading_state: String,
    pub favorited: bool,
    pub extraction_confident: bool,
    pub reading_progress: f64,
    pub tags: Vec<String>,
    pub category_id: Option<String>,
    pub font_size_override: Option<i64>,
    pub measure_override: Option<String>,
    pub leading_override: Option<String>,
    pub theme_override: Option<String>,
    pub font_override: Option<String>,
    pub updated_at: String,
}

fn sync_article_from_row(row: &Row) -> rusqlite::Result<SyncArticleRow> {
    let content_html_gz: Vec<u8> = row.get("content_html")?;
    let tags_raw: String = row.get("tags")?;
    Ok(SyncArticleRow {
        id: row.get("id")?,
        source_id: row.get("source_id")?,
        source_type: row.get("source_type")?,
        title: row.get("title")?,
        link: row.get("link")?,
        excerpt: row.get("excerpt")?,
        content_html: decompress_html(&content_html_gz),
        hero_image_path: row.get("hero_image_path")?,
        published_at: row.get("published_at")?,
        fetched_at: row.get("fetched_at")?,
        read_time_min: row.get("read_time_min")?,
        reading_state: row.get("reading_state")?,
        favorited: row.get("favorited")?,
        extraction_confident: row.get("extraction_confident")?,
        reading_progress: row.get("reading_progress")?,
        tags: parse_tags(tags_raw),
        category_id: row.get("category_id")?,
        font_size_override: row.get("font_size_override")?,
        measure_override: row.get("measure_override")?,
        leading_override: row.get("leading_override")?,
        theme_override: row.get("theme_override")?,
        font_override: row.get("font_override")?,
        updated_at: row.get("updated_at")?,
    })
}

const ARTICLE_SYNC_COLUMNS: &str = "id, source_id, source_type, title, link, excerpt,
    content_html, hero_image_path, published_at, fetched_at, read_time_min,
    reading_state, favorited, extraction_confident, reading_progress, tags,
    category_id, font_size_override, measure_override, leading_override,
    theme_override, font_override, updated_at";

/// Every article currently in the local DB, in sync's wire shape. No
/// paging (unlike `queries::list_articles_page`) — the manifest diff
/// needs to see every row's `updated_at`/`conflict_key` material in one
/// pass; article *content* is only pulled/pushed per-row afterward, so
/// this itself stays cheap even for a large library.
pub fn list_articles_for_sync(conn: &Connection) -> rusqlite::Result<Vec<SyncArticleRow>> {
    let mut stmt = conn.prepare(&format!("SELECT {ARTICLE_SYNC_COLUMNS} FROM articles"))?;
    let rows = stmt.query_map([], sync_article_from_row)?;
    rows.collect()
}

pub fn get_article_for_sync(
    conn: &Connection,
    id: &str,
) -> rusqlite::Result<Option<SyncArticleRow>> {
    conn.query_row(
        &format!("SELECT {ARTICLE_SYNC_COLUMNS} FROM articles WHERE id = ?1"),
        params![id],
        sync_article_from_row,
    )
    .optional()
}

/// Materializes a pulled remote article row locally — an insert if `id`
/// is new here, otherwise a full overwrite of every synced column
/// (the caller has already decided, via last-write-wins on `updated_at`
/// or the `conflict_key` collision merge, that the remote row should
/// win — see `remote_sync::conflict` and ARCHITECTURE.md's Sync
/// section). Requires `source_id`/`category_id` (if set) to already
/// exist locally — the sync engine pulls sources and categories before
/// articles for exactly this reason.
pub fn upsert_synced_article(conn: &Connection, row: &SyncArticleRow) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO articles (
            id, source_id, source_type, title, link, excerpt, content_html,
            hero_image_path, published_at, fetched_at, read_time_min,
            reading_state, favorited, extraction_confident, reading_progress,
            tags, category_id, font_size_override, measure_override,
            leading_override, theme_override, font_override, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                   ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)
         ON CONFLICT(id) DO UPDATE SET
            source_id = excluded.source_id,
            source_type = excluded.source_type,
            title = excluded.title,
            link = excluded.link,
            excerpt = excluded.excerpt,
            content_html = excluded.content_html,
            hero_image_path = excluded.hero_image_path,
            published_at = excluded.published_at,
            fetched_at = excluded.fetched_at,
            read_time_min = excluded.read_time_min,
            reading_state = excluded.reading_state,
            favorited = excluded.favorited,
            extraction_confident = excluded.extraction_confident,
            reading_progress = excluded.reading_progress,
            tags = excluded.tags,
            category_id = excluded.category_id,
            font_size_override = excluded.font_size_override,
            measure_override = excluded.measure_override,
            leading_override = excluded.leading_override,
            theme_override = excluded.theme_override,
            font_override = excluded.font_override,
            updated_at = excluded.updated_at",
        params![
            row.id,
            row.source_id,
            row.source_type,
            row.title,
            row.link,
            row.excerpt,
            compress_html(&row.content_html),
            row.hero_image_path,
            row.published_at,
            row.fetched_at,
            row.read_time_min,
            row.reading_state,
            row.favorited,
            row.extraction_confident,
            row.reading_progress,
            tags_to_json(&normalize_tags(&row.tags)),
            row.category_id,
            row.font_size_override,
            row.measure_override,
            row.leading_override,
            row.theme_override,
            row.font_override,
            row.updated_at,
        ],
    )?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncCategoryRow {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub created_at: String,
    pub updated_at: String,
}

fn sync_category_from_row(row: &Row) -> rusqlite::Result<SyncCategoryRow> {
    Ok(SyncCategoryRow {
        id: row.get("id")?,
        name: row.get("name")?,
        icon: row.get("icon")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub fn list_categories_for_sync(conn: &Connection) -> rusqlite::Result<Vec<SyncCategoryRow>> {
    let mut stmt = conn.prepare("SELECT id, name, icon, created_at, updated_at FROM categories")?;
    let rows = stmt.query_map([], sync_category_from_row)?;
    rows.collect()
}

/// Materializes a pulled remote category. Same insert-or-full-overwrite
/// shape as [`upsert_synced_article`] — see its doc comment.
pub fn upsert_synced_category(conn: &Connection, row: &SyncCategoryRow) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO categories (id, name, icon, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            icon = excluded.icon,
            updated_at = excluded.updated_at",
        params![row.id, row.name, row.icon, row.created_at, row.updated_at],
    )?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncSourceRow {
    pub id: String,
    pub name: String,
    pub feed_url: String,
    pub status: String,
    pub last_error: Option<String>,
    pub article_count: i64,
    pub last_synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn sync_source_from_row(row: &Row) -> rusqlite::Result<SyncSourceRow> {
    Ok(SyncSourceRow {
        id: row.get("id")?,
        name: row.get("name")?,
        // `sources.type` is CHECK-constrained to 'rss' only since schema
        // `V2` (see its doc comment) and `feed_url` is always populated
        // by every insert path (`db::schema`'s `V16` doc comment), so
        // unwrapping to an empty string rather than threading an
        // `Option` through the sync wire format is safe in practice —
        // and if it's ever wrong, an empty `conflict_key` just means
        // that source never dedupes against another, not a panic.
        feed_url: row
            .get::<_, Option<String>>("feed_url")?
            .unwrap_or_default(),
        status: row.get("status")?,
        last_error: row.get("last_error")?,
        article_count: row.get("article_count")?,
        last_synced_at: row.get("last_synced_at")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub fn list_sources_for_sync(conn: &Connection) -> rusqlite::Result<Vec<SyncSourceRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, feed_url, status, last_error, article_count, last_synced_at, created_at, updated_at
         FROM sources",
    )?;
    let rows = stmt.query_map([], sync_source_from_row)?;
    rows.collect()
}

/// Materializes a pulled remote source. `article_count` is taken as-is
/// from the remote row on insert, but deliberately left alone on
/// conflict — it's a local cache each device's own capture activity
/// keeps current (`queries::insert_captured_article`/`delete_article`),
/// and overwriting it here would fight whichever device most recently
/// captured against this source.
pub fn upsert_synced_source(conn: &Connection, row: &SyncSourceRow) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO sources (id, name, type, feed_url, status, last_error, article_count, last_synced_at, created_at, updated_at)
         VALUES (?1, ?2, 'rss', ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            feed_url = excluded.feed_url,
            status = excluded.status,
            last_error = excluded.last_error,
            last_synced_at = excluded.last_synced_at,
            updated_at = excluded.updated_at",
        params![
            row.id,
            row.name,
            row.feed_url,
            row.status,
            row.last_error,
            row.article_count,
            row.last_synced_at,
            row.created_at,
            row.updated_at,
        ],
    )?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalTombstone {
    pub entity_type: String,
    pub entity_id: String,
    pub deleted_at: String,
}

/// Every tombstone this device knows about (its own deletes, plus
/// whatever it has already pulled from previous syncs — `sync_tombstones`
/// carries both, there is no separate "pending push" queue). Pushed
/// wholesale into the manifest on every sync; see
/// `manifest::Manifest::purge_expired_tombstones` for how the set is kept
/// bounded over time.
pub fn list_local_tombstones(conn: &Connection) -> rusqlite::Result<Vec<LocalTombstone>> {
    let mut stmt =
        conn.prepare("SELECT entity_type, entity_id, deleted_at FROM sync_tombstones")?;
    let rows = stmt.query_map([], |row| {
        Ok(LocalTombstone {
            entity_type: row.get("entity_type")?,
            entity_id: row.get("entity_id")?,
            deleted_at: row.get("deleted_at")?,
        })
    })?;
    rows.collect()
}

/// Records a tombstone this device learned about from a remote manifest
/// (as opposed to `queries::delete_article`/`delete_category`/
/// `remove_source`, which record one from a local user action). Exposed
/// separately from those so the sync engine can record a remote
/// tombstone's *original* `deleted_at` rather than "now". Always
/// overwrites unconditionally rather than reconciling with whatever was
/// already there: only one origin device ever creates a given tombstone,
/// so every other device applying it is backdating to that same fixed
/// origin timestamp. This matters right after `queries::delete_article`
/// et al. (see `apply_tombstone` below), which already wrote their own
/// "now" into this same row and needs it replaced with the real
/// original, not merged with it.
pub fn record_remote_tombstone(
    conn: &Connection,
    entity_type: &str,
    entity_id: &str,
    deleted_at: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO sync_tombstones (entity_type, entity_id, deleted_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(entity_type, entity_id) DO UPDATE SET deleted_at = excluded.deleted_at",
        params![entity_type, entity_id, deleted_at],
    )?;
    Ok(())
}

/// Applies an incoming tombstone locally: deletes the entity via the
/// exact same path a local user delete would take
/// (`queries::delete_article`/`delete_category`/`remove_source`), then
/// records the tombstone under its original `deleted_at`. Reusing those
/// functions rather than a separate delete path means an applied
/// tombstone gets the same `article_count` bookkeeping, the same
/// `ON DELETE SET NULL` category un-linking, and the same "deleting an
/// already-gone id is a no-op" idempotency every local delete already
/// has. Returns the article's `hero_image_path` when an article was
/// deleted, so the caller can clean up `content/<id>/` and the hero
/// thumbnail the same way a local delete's caller would — `None` for
/// every other entity type, or when the id was already gone locally.
pub fn apply_tombstone(
    conn: &Connection,
    entity_type: &str,
    entity_id: &str,
    deleted_at: &str,
) -> rusqlite::Result<Option<Option<String>>> {
    let hero_image_path: Option<Option<String>> = match entity_type {
        "article" => {
            super::queries::delete_article(conn, entity_id)?.map(|files| files.hero_image_path)
        }
        "category" => {
            super::queries::delete_category(conn, entity_id)?;
            None
        }
        "source" => {
            super::queries::remove_source(conn, entity_id)?;
            None
        }
        _ => None,
    };
    record_remote_tombstone(conn, entity_type, entity_id, deleted_at)?;
    Ok(hero_image_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::LocalCaptureOutput;

    fn migrated_conn() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        crate::db::schema::migrate(&mut conn).expect("migrate");
        conn
    }

    fn sample_output(link: &str) -> LocalCaptureOutput {
        LocalCaptureOutput {
            title: "Title".to_string(),
            link: link.to_string(),
            final_url: link.to_string(),
            excerpt: "excerpt".to_string(),
            content_html: "<p>hello</p>".to_string(),
            published_at: None,
            read_time_min: 3,
            hero_image_path: None,
            extraction_confident: true,
        }
    }

    #[test]
    fn round_trips_an_article_through_list_and_upsert() {
        let conn = migrated_conn();
        super::super::queries::insert_captured_article(
            &conn,
            "art-1",
            None,
            "direct",
            &sample_output("https://example.com/a"),
            &["tag-a".to_string()],
        )
        .unwrap();

        let mut rows = list_articles_for_sync(&conn).unwrap();
        assert_eq!(rows.len(), 1);
        let mut row = rows.remove(0);
        assert_eq!(row.content_html, "<p>hello</p>");

        // Simulate a remote edit (e.g. tagged on another device) and
        // apply it as an incoming pull.
        row.tags = vec!["tag-a".to_string(), "tag-b".to_string()];
        row.favorited = true;
        upsert_synced_article(&conn, &row).unwrap();

        let refreshed = get_article_for_sync(&conn, "art-1").unwrap().unwrap();
        assert_eq!(refreshed.tags, vec!["tag-a", "tag-b"]);
        assert!(refreshed.favorited);
        assert_eq!(refreshed.content_html, "<p>hello</p>");
    }

    #[test]
    fn upsert_synced_article_inserts_a_brand_new_id() {
        let conn = migrated_conn();
        let row = SyncArticleRow {
            id: "art-remote".to_string(),
            source_id: None,
            source_type: "direct".to_string(),
            title: "Remote".to_string(),
            link: "https://example.com/remote".to_string(),
            excerpt: "e".to_string(),
            content_html: "<p>remote</p>".to_string(),
            hero_image_path: None,
            published_at: None,
            fetched_at: "2026-01-01T00:00:00Z".to_string(),
            read_time_min: 2,
            reading_state: "unread".to_string(),
            favorited: false,
            extraction_confident: true,
            reading_progress: 0.0,
            tags: vec![],
            category_id: None,
            font_size_override: None,
            measure_override: None,
            leading_override: None,
            theme_override: None,
            font_override: None,
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };

        upsert_synced_article(&conn, &row).unwrap();

        let fetched = get_article_for_sync(&conn, "art-remote").unwrap().unwrap();
        assert_eq!(fetched, row);
    }

    #[test]
    fn round_trips_a_category_and_a_source() {
        let conn = migrated_conn();
        let category = crate::db::queries::create_category(&conn, "Recipes").unwrap();
        let source =
            crate::db::queries::insert_rss_source(&conn, "Feed", "https://example.com/feed.xml")
                .unwrap();

        let categories = list_categories_for_sync(&conn).unwrap();
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].id, category.id);

        let sources = list_sources_for_sync(&conn).unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].feed_url, "https://example.com/feed.xml");
        assert_eq!(sources[0].id, source.id);
    }

    #[test]
    fn apply_tombstone_deletes_the_article_and_records_its_original_timestamp() {
        let conn = migrated_conn();
        crate::db::queries::insert_captured_article(
            &conn,
            "art-1",
            None,
            "direct",
            &sample_output("https://example.com/a"),
            &[],
        )
        .unwrap();

        apply_tombstone(&conn, "article", "art-1", "2020-01-01T00:00:00Z").unwrap();

        assert!(get_article_for_sync(&conn, "art-1").unwrap().is_none());
        let tombstones = list_local_tombstones(&conn).unwrap();
        assert_eq!(tombstones.len(), 1);
        assert_eq!(tombstones[0].deleted_at, "2020-01-01T00:00:00Z");
    }

    #[test]
    fn apply_tombstone_on_an_already_gone_id_is_a_no_op_that_still_records_the_tombstone() {
        let conn = migrated_conn();
        let result = apply_tombstone(&conn, "article", "never-existed", "2020-01-01T00:00:00Z");
        assert!(result.is_ok());
        assert_eq!(list_local_tombstones(&conn).unwrap().len(), 1);
    }

    #[test]
    fn record_remote_tombstone_overwrites_unconditionally() {
        let conn = migrated_conn();
        record_remote_tombstone(&conn, "article", "art-1", "2020-01-01T00:00:00Z").unwrap();
        record_remote_tombstone(&conn, "article", "art-1", "2019-01-01T00:00:00Z").unwrap();

        let tombstones = list_local_tombstones(&conn).unwrap();
        assert_eq!(
            tombstones[0].deleted_at, "2019-01-01T00:00:00Z",
            "the most recent call must win outright, matching apply_tombstone's backdate-after-delete use"
        );
    }
}

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, Row, params};
use uuid::Uuid;

use crate::capture::CaptureOutput;
use crate::models::{ArticleDetail, ArticleSummary, Settings, Source};

fn article_summary_from_row(row: &Row) -> rusqlite::Result<ArticleSummary> {
    Ok(ArticleSummary {
        id: row.get("id")?,
        title: row.get("title")?,
        source_name: row.get("source_name")?,
        source_type: row.get("source_type")?,
        excerpt: row.get("excerpt")?,
        hero_image_path: row.get("hero_image_path")?,
        published_at: row.get("published_at")?,
        read_time_min: row.get("read_time_min")?,
        unread: row.get::<_, i64>("unread")? != 0,
        favorited: row.get::<_, i64>("favorited")? != 0,
    })
}

/// A cheap pre-check used to skip capturing (fetching + localizing +
/// archiving) an article whose link is already known, before doing any of
/// that work. This is an optimization only, not the dedup guarantee — two
/// concurrent syncs racing on the same brand-new link can both pass this
/// check and both attempt to insert; [`insert_captured_article`]'s
/// `UNIQUE(link)` index plus `INSERT OR IGNORE` is what actually makes
/// that safe.
pub fn article_link_exists(conn: &Connection, link: &str) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT 1 FROM articles WHERE link = ?1 LIMIT 1",
        params![link],
        |_| Ok(()),
    )
    .optional()
    .map(|r| r.is_some())
}

/// Inserts a freshly captured article under the given `id` (the same id
/// `capture_article` was called with, since that's what its ZIM/hero-image
/// file paths are named after). `source_id` is `None` for direct-link
/// captures (they aren't tied to a recurring source).
///
/// Returns `true` if the row was actually inserted, `false` if
/// `output.link` already existed and the `UNIQUE(link)` index silently
/// absorbed the insert via `OR IGNORE` — the authoritative dedup signal
/// (atomic, unlike the best-effort [`article_link_exists`] pre-check a
/// caller may have already used to skip the capture work entirely).
pub fn insert_captured_article(
    conn: &Connection,
    id: &str,
    source_id: Option<&str>,
    source_name: &str,
    source_type: &str,
    output: &CaptureOutput,
) -> rusqlite::Result<bool> {
    let now = Utc::now().to_rfc3339();
    let inserted = conn.execute(
        "INSERT OR IGNORE INTO articles (
            id, source_id, source_name, source_type, title, link, excerpt,
            content_html, hero_image_path, published_at, fetched_at,
            read_time_min, unread, favorited, zim_path, zim_main_path,
            extraction_confident, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 1, 0, ?13, ?14, ?15, ?16)",
        params![
            id,
            source_id,
            source_name,
            source_type,
            output.title,
            output.link,
            output.excerpt,
            output.content_html,
            output.hero_image_path,
            output.published_at,
            now,
            output.read_time_min,
            output.zim_path,
            output.zim_main_path,
            output.extraction_confident,
            now,
        ],
    )? > 0;
    if inserted && let Some(sid) = source_id {
        conn.execute(
            "UPDATE sources SET article_count = article_count + 1, updated_at = ?2 WHERE id = ?1",
            params![sid, now],
        )?;
    }
    Ok(inserted)
}

pub fn list_articles(conn: &Connection) -> rusqlite::Result<Vec<ArticleSummary>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, source_name, source_type, excerpt, hero_image_path,
                published_at, read_time_min, unread, favorited
         FROM articles ORDER BY fetched_at DESC",
    )?;
    let rows = stmt.query_map([], article_summary_from_row)?;
    rows.collect()
}

pub fn get_article(conn: &Connection, id: &str) -> rusqlite::Result<Option<ArticleDetail>> {
    conn.query_row(
        "SELECT id, title, source_name, source_type, excerpt, hero_image_path,
                published_at, read_time_min, unread, favorited, link, content_html
         FROM articles WHERE id = ?1",
        params![id],
        |row| {
            Ok(ArticleDetail {
                id: row.get("id")?,
                title: row.get("title")?,
                source_name: row.get("source_name")?,
                source_type: row.get("source_type")?,
                excerpt: row.get("excerpt")?,
                hero_image_path: row.get("hero_image_path")?,
                published_at: row.get("published_at")?,
                read_time_min: row.get("read_time_min")?,
                unread: row.get::<_, i64>("unread")? != 0,
                favorited: row.get::<_, i64>("favorited")? != 0,
                link: row.get("link")?,
                content_html: row.get("content_html")?,
            })
        },
    )
    .optional()
}

/// Looks up an article's summary by its (cleaned) link — used when
/// [`insert_captured_article`] reports a duplicate, so a caller like
/// `direct_link::capture_direct_link` can return the article that's
/// actually stored under that link instead of one describing a row that
/// was never inserted.
pub fn get_article_summary_by_link(
    conn: &Connection,
    link: &str,
) -> rusqlite::Result<Option<ArticleSummary>> {
    conn.query_row(
        "SELECT id, title, source_name, source_type, excerpt, hero_image_path,
                published_at, read_time_min, unread, favorited
         FROM articles WHERE link = ?1",
        params![link],
        article_summary_from_row,
    )
    .optional()
}

pub fn mark_read(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE articles SET unread = 0, updated_at = ?2 WHERE id = ?1",
        params![id, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn toggle_favorite(conn: &Connection, id: &str) -> rusqlite::Result<bool> {
    conn.execute(
        "UPDATE articles SET favorited = 1 - favorited, updated_at = ?2 WHERE id = ?1",
        params![id, Utc::now().to_rfc3339()],
    )?;
    conn.query_row(
        "SELECT favorited FROM articles WHERE id = ?1",
        params![id],
        |row| row.get::<_, i64>(0),
    )
    .map(|v| v != 0)
}

fn source_from_row(row: &Row) -> rusqlite::Result<Source> {
    Ok(Source {
        id: row.get("id")?,
        name: row.get("name")?,
        source_type: row.get("type")?,
        feed_url: row.get("feed_url")?,
        status: row.get("status")?,
        last_error: row.get("last_error")?,
        article_count: row.get("article_count")?,
        last_synced_at: row.get("last_synced_at")?,
        created_at: row.get("created_at")?,
    })
}

pub fn list_sources(conn: &Connection) -> rusqlite::Result<Vec<Source>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, type, feed_url, status, last_error, article_count, last_synced_at, created_at
         FROM sources ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], source_from_row)?;
    rows.collect()
}

pub fn get_source(conn: &Connection, id: &str) -> rusqlite::Result<Option<Source>> {
    conn.query_row(
        "SELECT id, name, type, feed_url, status, last_error, article_count, last_synced_at, created_at
         FROM sources WHERE id = ?1",
        params![id],
        source_from_row,
    )
    .optional()
}

pub fn insert_rss_source(conn: &Connection, name: &str, feed_url: &str) -> rusqlite::Result<Source> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO sources (id, name, type, feed_url, status, article_count, created_at, updated_at)
         VALUES (?1, ?2, 'rss', ?3, 'active', 0, ?4, ?4)",
        params![id, name, feed_url, created_at],
    )?;
    Ok(Source {
        id,
        name: name.to_string(),
        source_type: "rss".to_string(),
        feed_url: Some(feed_url.to_string()),
        status: "active".to_string(),
        last_error: None,
        article_count: 0,
        last_synced_at: None,
        created_at,
    })
}

pub fn toggle_source_pause(conn: &Connection, id: &str) -> rusqlite::Result<Source> {
    conn.execute(
        "UPDATE sources SET status = CASE status WHEN 'paused' THEN 'active' ELSE 'paused' END,
                            updated_at = ?2
         WHERE id = ?1 AND status != 'error'",
        params![id, Utc::now().to_rfc3339()],
    )?;
    get_source(conn, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn remove_source(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM sources WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn mark_source_synced(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE sources SET last_synced_at = ?1, status = 'active', last_error = NULL, updated_at = ?1
         WHERE id = ?2",
        params![now, id],
    )?;
    Ok(())
}

pub fn mark_source_error(conn: &Connection, id: &str, error: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE sources SET status = 'error', last_error = ?1, updated_at = ?3 WHERE id = ?2",
        params![error, id, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn get_settings(conn: &Connection) -> rusqlite::Result<Settings> {
    let mut settings = Settings::default();
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (key, value) = row?;
        match key.as_str() {
            "default_font_size" => settings.default_font_size = value,
            "default_library_view" => settings.default_library_view = value,
            "autosync" => settings.autosync = value == "true",
            _ => {}
        }
    }
    Ok(settings)
}

pub fn update_settings(conn: &Connection, settings: &Settings) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('default_font_size', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![settings.default_font_size],
    )?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('default_library_view', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![settings.default_library_view],
    )?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('autosync', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![if settings.autosync { "true" } else { "false" }],
    )?;
    Ok(())
}

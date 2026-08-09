use std::collections::HashSet;

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, Row, params};
use uuid::Uuid;

use crate::capture::LocalCaptureOutput;
use crate::models::{ArticleDetail, ArticleSummary, Settings, Source};

/// `tags` is stored as a JSON array string; a row with anything other
/// than a valid JSON array (shouldn't happen — only this module writes
/// the column) is treated as untagged rather than failing the whole
/// query.
fn parse_tags(raw: String) -> Vec<String> {
    serde_json::from_str(&raw).unwrap_or_default()
}

fn tags_to_json(tags: &[String]) -> String {
    serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string())
}

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
        reading_state: row.get("reading_state")?,
        favorited: row.get::<_, i64>("favorited")? != 0,
        reading_progress: row.get("reading_progress")?,
        tags: parse_tags(row.get("tags")?),
    })
}

const ARTICLE_SUMMARY_COLUMNS: &str = "id, title, source_name, source_type, excerpt, hero_image_path,
                published_at, read_time_min, reading_state, favorited, reading_progress, tags";

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
/// `capture_local` was called with, since that's what its content-zim/
/// hero-image file paths are named after). `source_id` is `None` for
/// direct-link captures (they aren't tied to a recurring source).
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
    output: &LocalCaptureOutput,
    tags: &[String],
) -> rusqlite::Result<bool> {
    let now = Utc::now().to_rfc3339();
    let inserted = conn.execute(
        "INSERT OR IGNORE INTO articles (
            id, source_id, source_name, source_type, title, link, excerpt,
            content_html, hero_image_path, published_at, fetched_at,
            read_time_min, favorited, content_zim_path,
            extraction_confident, tags, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0, ?13, ?14, ?15, ?16)",
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
            output.content_zim_path,
            output.extraction_confident,
            tags_to_json(tags),
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

/// Overwrites an existing article's *readable* content in place, for
/// `recapture_article` — re-running the local capture pipeline against the
/// same id (and thus the same on-disk content-zim/hero-image paths, which
/// the caller has already overwritten by this point). `reading_state`/
/// `favorited`/`reading_progress` are deliberately left untouched: a
/// re-capture refreshes the *content*, not the reader's relationship to
/// it. `link` is updated too — if that collides with another article's
/// `UNIQUE(link)`, this fails with a constraint error rather than
/// silently corrupting either row, which is the right outcome for a
/// manual, occasional action.
///
/// The caller is responsible for evicting any stale content-zim cache
/// entry (the on-disk file at this id's `content_zim_path` was already
/// overwritten by this point).
pub fn update_captured_article(
    conn: &Connection,
    id: &str,
    output: &LocalCaptureOutput,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE articles SET
            title = ?2, link = ?3, excerpt = ?4, content_html = ?5,
            hero_image_path = ?6, published_at = ?7, read_time_min = ?8,
            content_zim_path = ?9, extraction_confident = ?10,
            updated_at = ?11
         WHERE id = ?1",
        params![
            id,
            output.title,
            output.link,
            output.excerpt,
            output.content_html,
            output.hero_image_path,
            output.published_at,
            output.read_time_min,
            output.content_zim_path,
            output.extraction_confident,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub fn list_articles(conn: &Connection) -> rusqlite::Result<Vec<ArticleSummary>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {ARTICLE_SUMMARY_COLUMNS} FROM articles ORDER BY fetched_at DESC"
    ))?;
    let rows = stmt.query_map([], article_summary_from_row)?;
    rows.collect()
}

pub fn get_article(conn: &Connection, id: &str) -> rusqlite::Result<Option<ArticleDetail>> {
    conn.query_row(
        "SELECT id, title, source_name, source_type, excerpt, hero_image_path,
                published_at, read_time_min, reading_state, favorited, link, content_html,
                extraction_confident, reading_progress, tags
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
                reading_state: row.get("reading_state")?,
                favorited: row.get::<_, i64>("favorited")? != 0,
                link: row.get("link")?,
                content_html: row.get("content_html")?,
                extraction_confident: row.get::<_, i64>("extraction_confident")? != 0,
                reading_progress: row.get("reading_progress")?,
                tags: parse_tags(row.get("tags")?),
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
        &format!("SELECT {ARTICLE_SUMMARY_COLUMNS} FROM articles WHERE link = ?1"),
        params![link],
        article_summary_from_row,
    )
    .optional()
}

/// An article's persistent, never-evicted content zim path (relative to
/// `data_dir`), nullable. Looking this up by id doubles as the `zim://`
/// protocol handler's traversal guard — only an id that's actually a
/// stored article's row resolves to a real file, so an arbitrary/forged
/// id in a `zim://` request can't reach any other file under `content/`.
pub fn get_article_content_zim_path(conn: &Connection, id: &str) -> rusqlite::Result<Option<Option<String>>> {
    conn.query_row(
        "SELECT content_zim_path FROM articles WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )
    .optional()
}

/// Persists the reader's scroll-fraction progress for an article, called
/// on a debounce from the reader's scroll handler.
pub fn save_reading_progress(conn: &Connection, id: &str, progress: f64) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE articles SET reading_progress = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, progress.clamp(0.0, 1.0), Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

/// On-disk file paths (relative to `data_dir`) an about-to-be-deleted
/// article owns, so the caller can remove them after the row itself is
/// gone.
pub struct DeletedArticleFiles {
    pub content_zim_path: Option<String>,
    pub hero_image_path: Option<String>,
}

/// Deletes an article and decrements its source's `article_count` (floored
/// at 0). Returns `None` if `id` didn't match any row — deleting an
/// already-gone article is treated as a no-op success by the caller, not
/// an error. `remove_source` deliberately does *not* cascade to articles
/// (`ON DELETE SET NULL` — read-later semantics: removing a feed doesn't
/// discard what you already saved from it), so this is the only path that
/// ever deletes an article row.
pub fn delete_article(conn: &Connection, id: &str) -> rusqlite::Result<Option<DeletedArticleFiles>> {
    let row = conn
        .query_row(
            "SELECT source_id, content_zim_path, hero_image_path FROM articles WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>("source_id")?,
                    row.get::<_, Option<String>>("content_zim_path")?,
                    row.get::<_, Option<String>>("hero_image_path")?,
                ))
            },
        )
        .optional()?;

    let Some((source_id, content_zim_path, hero_image_path)) = row else {
        return Ok(None);
    };

    conn.execute("DELETE FROM articles WHERE id = ?1", params![id])?;

    if let Some(sid) = source_id {
        conn.execute(
            "UPDATE sources SET article_count = MAX(0, article_count - 1), updated_at = ?2 WHERE id = ?1",
            params![sid, Utc::now().to_rfc3339()],
        )?;
    }

    Ok(Some(DeletedArticleFiles {
        content_zim_path,
        hero_image_path,
    }))
}

/// Every `content_zim_path`/`hero_image_path` currently referenced by a
/// live article row — the startup orphan sweep (`gc::sweep_orphaned_files`)
/// diffs this against what's actually on disk under `content/`/`media/`
/// and removes whatever isn't in this set.
pub fn list_referenced_files(conn: &Connection) -> rusqlite::Result<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT content_zim_path, hero_image_path FROM articles")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, Option<String>>(0)?,
            row.get::<_, Option<String>>(1)?,
        ))
    })?;
    let mut referenced = HashSet::new();
    for row in rows {
        let (content_zim_path, hero_image_path) = row?;
        for path in [content_zim_path, hero_image_path].into_iter().flatten() {
            referenced.insert(path);
        }
    }
    Ok(referenced)
}

/// Transitions an article into `reading` — called when the reader opens
/// it, whether it was previously `unread` or `read` (reopening a finished
/// article resumes it).
pub fn transition_to_reading(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE articles SET reading_state = 'reading', updated_at = ?2 WHERE id = ?1",
        params![id, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

/// Transitions an article into `read` — only ever the manual "mark as
/// read" action, never automatic.
pub fn transition_to_read(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE articles SET reading_state = 'read', updated_at = ?2 WHERE id = ?1",
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

/// Replaces a source's placeholder name (the raw feed URL, set at add-time
/// before the feed's own title was known) with its real title — but only
/// the first time: the `WHERE name = feed_url` guard means this becomes a
/// no-op on every later sync once the rename has happened once, and would
/// never overwrite a name the user has since customized (no rename UI
/// exists yet, but this guard is what makes adding one safe later).
pub fn set_source_name_if_default(
    conn: &Connection,
    id: &str,
    title: &str,
    feed_url: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE sources SET name = ?1, updated_at = ?4 WHERE id = ?2 AND name = ?3",
        params![title, id, feed_url, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

/// Toggles a source between `active` and `paused`. Deliberately has no
/// `status != 'error'` guard (an earlier version did): an errored source
/// must be pausable (stop retrying a feed that's broken) and resumable
/// (try again) just like any other, and this is the only UI path back to
/// `active` from `error` short of a successful sync — without it, an
/// errored source was stuck until removed and re-added.
pub fn toggle_source_pause(conn: &Connection, id: &str) -> rusqlite::Result<Source> {
    conn.execute(
        "UPDATE sources SET status = CASE status WHEN 'paused' THEN 'active' ELSE 'paused' END,
                            updated_at = ?2
         WHERE id = ?1",
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
            "reader_font_size" => {
                if let Ok(size) = value.parse() {
                    settings.reader_font_size = size;
                }
            }
            "reader_measure" => settings.reader_measure = value,
            "reader_leading" => settings.reader_leading = value,
            "app_theme" => settings.app_theme = value,
            "reader_theme" => settings.reader_theme = value,
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
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('reader_font_size', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![settings.reader_font_size.to_string()],
    )?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('reader_measure', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![settings.reader_measure],
    )?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('reader_leading', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![settings.reader_leading],
    )?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('app_theme', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![settings.app_theme],
    )?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('reader_theme', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![settings.reader_theme],
    )?;
    Ok(())
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

    fn sample_capture_output(link: &str) -> LocalCaptureOutput {
        LocalCaptureOutput {
            title: "Title".to_string(),
            link: link.to_string(),
            final_url: link.to_string(),
            excerpt: "excerpt".to_string(),
            content_html: "<p>content</p>".to_string(),
            published_at: None,
            read_time_min: 3,
            hero_image_path: None,
            content_zim_path: "content/x.zim".to_string(),
            extraction_confident: true,
        }
    }

    #[test]
    fn toggle_source_pause_recovers_a_source_from_error_status() {
        let conn = migrated_conn();
        let source = insert_rss_source(&conn, "Feed", "https://example.com/feed.xml").unwrap();
        mark_source_error(&conn, &source.id, "boom").unwrap();
        assert_eq!(get_source(&conn, &source.id).unwrap().unwrap().status, "error");

        // Errored source must be pausable...
        let paused = toggle_source_pause(&conn, &source.id).unwrap();
        assert_eq!(paused.status, "paused");

        // ...and resumable back to active, the only UI path out of `error`
        // short of a successful sync.
        let resumed = toggle_source_pause(&conn, &source.id).unwrap();
        assert_eq!(resumed.status, "active");
    }

    #[test]
    fn mark_source_synced_clears_error_status_and_message() {
        let conn = migrated_conn();
        let source = insert_rss_source(&conn, "Feed", "https://example.com/feed.xml").unwrap();
        mark_source_error(&conn, &source.id, "boom").unwrap();

        mark_source_synced(&conn, &source.id).unwrap();

        let refreshed = get_source(&conn, &source.id).unwrap().unwrap();
        assert_eq!(refreshed.status, "active");
        assert_eq!(refreshed.last_error, None);
    }

    #[test]
    fn delete_article_removes_row_decrements_source_count_and_returns_file_paths() {
        let conn = migrated_conn();
        let source = insert_rss_source(&conn, "Feed", "https://example.com/feed.xml").unwrap();
        let output = sample_capture_output("https://example.com/article");
        insert_captured_article(&conn, "art-1", Some(&source.id), &source.name, "rss", &output, &[]).unwrap();
        assert_eq!(get_source(&conn, &source.id).unwrap().unwrap().article_count, 1);

        let deleted = delete_article(&conn, "art-1").unwrap().expect("row existed");
        assert_eq!(deleted.content_zim_path.as_deref(), Some("content/x.zim"));
        assert_eq!(deleted.hero_image_path, None);

        assert!(get_article(&conn, "art-1").unwrap().is_none());
        assert_eq!(get_source(&conn, &source.id).unwrap().unwrap().article_count, 0);
    }

    #[test]
    fn delete_article_is_a_no_op_for_an_unknown_id() {
        let conn = migrated_conn();
        assert!(delete_article(&conn, "does-not-exist").unwrap().is_none());
    }

    #[test]
    fn set_source_name_if_default_only_replaces_the_placeholder_once() {
        let conn = migrated_conn();
        let feed_url = "https://example.com/feed.xml";
        let source = insert_rss_source(&conn, feed_url, feed_url).unwrap();
        assert_eq!(source.name, feed_url);

        set_source_name_if_default(&conn, &source.id, "Real Feed Title", feed_url).unwrap();
        assert_eq!(get_source(&conn, &source.id).unwrap().unwrap().name, "Real Feed Title");

        // A later call — even with a *different* title — must not
        // overwrite a name that's no longer the placeholder (this is also
        // what protects a future user-set custom name).
        set_source_name_if_default(&conn, &source.id, "Some Other Title", feed_url).unwrap();
        assert_eq!(get_source(&conn, &source.id).unwrap().unwrap().name, "Real Feed Title");
    }
}

use std::collections::HashSet;

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, Row, params};
use uuid::Uuid;

use crate::capture::LocalCaptureOutput;
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
        reading_state: row.get("reading_state")?,
        favorited: row.get::<_, i64>("favorited")? != 0,
        reading_progress: row.get("reading_progress")?,
        archive_status: row.get("archive_status")?,
        archive_source: row.get("archive_source")?,
        archive_available_locally: row.get::<_, Option<String>>("zim_path")?.is_some(),
    })
}

const ARTICLE_SUMMARY_COLUMNS: &str = "id, title, source_name, source_type, excerpt, hero_image_path,
                published_at, read_time_min, reading_state, favorited, reading_progress,
                archive_status, archive_source, zim_path";

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
/// The full archive isn't captured yet at insert time — `zim_path`/
/// `zim_main_path` stay `NULL` and `archive_status` defaults to `pending`
/// (schema default; not set explicitly here) until the archive reconciler
/// observes the server-side job finish. The caller is responsible for
/// submitting that job (`crate::archive_client::submit_capture_job`) right
/// after this returns.
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
) -> rusqlite::Result<bool> {
    let now = Utc::now().to_rfc3339();
    let inserted = conn.execute(
        "INSERT OR IGNORE INTO articles (
            id, source_id, source_name, source_type, title, link, excerpt,
            content_html, hero_image_path, published_at, fetched_at,
            read_time_min, favorited, content_zim_path,
            extraction_confident, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0, ?13, ?14, ?15)",
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
/// Also resets the full-archive bookkeeping to `server`/`pending` and
/// nulls `zim_path`/`zim_main_path` — the caller is responsible for
/// re-submitting a server capture job and evicting any stale local/
/// content-zim cache entries, matching a fresh capture's lifecycle even
/// for an article that predates server-side archiving (`local_legacy`).
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
            zim_path = NULL, zim_main_path = NULL,
            archive_source = 'server', archive_status = 'pending',
            archive_last_error = NULL, updated_at = ?11
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
                zim_main_path, extraction_confident, reading_progress, archive_status,
                archive_source, zim_path
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
                zim_main_path: row.get("zim_main_path")?,
                extraction_confident: row.get::<_, i64>("extraction_confident")? != 0,
                reading_progress: row.get("reading_progress")?,
                archive_status: row.get("archive_status")?,
                archive_source: row.get("archive_source")?,
                archive_available_locally: row.get::<_, Option<String>>("zim_path")?.is_some(),
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

/// An article's two possible on-disk ZIM paths (both relative to
/// `data_dir`, both nullable): the persistent, never-evicted content zim
/// (readable-view images) and the ephemeral full archive (only present
/// while cached locally). Looking this up by id doubles as the `zim://`
/// protocol handler's traversal guard — only an id that's actually a
/// stored article's row resolves to real files, so an arbitrary/forged id
/// in a `zim://` request can't reach any other file under `content/`/
/// `archives/`.
pub struct ArticleZimPaths {
    pub content_zim_path: Option<String>,
    pub zim_path: Option<String>,
}

pub fn get_article_zim_paths(conn: &Connection, id: &str) -> rusqlite::Result<Option<ArticleZimPaths>> {
    conn.query_row(
        "SELECT content_zim_path, zim_path FROM articles WHERE id = ?1",
        params![id],
        |row| {
            Ok(ArticleZimPaths {
                content_zim_path: row.get(0)?,
                zim_path: row.get(1)?,
            })
        },
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
/// gone. `zim_path` is `None` when the full archive wasn't cached locally
/// at delete time (evicted, or never downloaded) — nothing to remove for
/// it in that case, the server retains its own copy independently.
pub struct DeletedArticleFiles {
    pub zim_path: Option<String>,
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
            "SELECT source_id, zim_path, content_zim_path, hero_image_path FROM articles WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>("source_id")?,
                    row.get::<_, Option<String>>("zim_path")?,
                    row.get::<_, Option<String>>("content_zim_path")?,
                    row.get::<_, Option<String>>("hero_image_path")?,
                ))
            },
        )
        .optional()?;

    let Some((source_id, zim_path, content_zim_path, hero_image_path)) = row else {
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
        zim_path,
        content_zim_path,
        hero_image_path,
    }))
}

/// Every `zim_path`/`content_zim_path`/`hero_image_path` currently
/// referenced by a live article row — the startup orphan sweep
/// (`gc::sweep_orphaned_files`) diffs this against what's actually on disk
/// under `archives/`/`media/` and removes whatever isn't in this set.
/// `zim_path` is nullable now (a `server`-sourced archive not currently
/// cached locally has none) — this is not optional to include correctly:
/// omitting `content_zim_path` here would make the very next orphan sweep
/// delete every persistent readable-view image store it finds.
pub fn list_referenced_files(conn: &Connection) -> rusqlite::Result<HashSet<String>> {
    let mut stmt =
        conn.prepare("SELECT zim_path, content_zim_path, hero_image_path FROM articles")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, Option<String>>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })?;
    let mut referenced = HashSet::new();
    for row in rows {
        let (zim_path, content_zim_path, hero_image_path) = row?;
        for path in [zim_path, content_zim_path, hero_image_path].into_iter().flatten() {
            referenced.insert(path);
        }
    }
    Ok(referenced)
}

/// Transitions an article into `reading` — called when the reader opens
/// it, whether it was previously `unread` or `read` (reopening a finished
/// article resumes it). Touches `archive_last_opened_at` (the local
/// archive-cache LRU's recency key) only for `server`-sourced rows;
/// `local_legacy` archives are never evicted, so they don't participate.
pub fn transition_to_reading(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE articles SET
            reading_state = 'reading',
            archive_last_opened_at = CASE WHEN archive_source = 'server' THEN ?2 ELSE archive_last_opened_at END,
            updated_at = ?2
         WHERE id = ?1",
        params![id, now],
    )?;
    Ok(())
}

/// Transitions an article into `read` — only ever the manual "mark as
/// read" action, never automatic. Does not touch any archive file; the
/// caller is responsible for evicting the locally cached full archive (if
/// any) after this, mirroring the LRU sweep's own "null the column, then
/// remove the file" split.
pub fn transition_to_read(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE articles SET reading_state = 'read', updated_at = ?2 WHERE id = ?1",
        params![id, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

/// Every `server`-sourced article whose archive job is still `pending` —
/// what the archive reconciler polls each tick. `link` (rather than the
/// original `final_url` `capture_local` saw, which isn't persisted) is
/// what gets resubmitted if the server has no record of the job at all.
pub fn list_pending_archive_jobs(conn: &Connection) -> rusqlite::Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT id, link FROM articles WHERE archive_source = 'server' AND archive_status = 'pending'",
    )?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect()
}

/// Records that a `server`-sourced article's full-page capture finished —
/// `zim_main_path` is the server's own reported entry path (never
/// recomputed locally; server-side Chromium can follow redirects a plain
/// fetch never sees). Does not cache the bytes locally — that's a
/// separate step, only taken for articles currently being read.
pub fn set_archive_ready(conn: &Connection, id: &str, zim_main_path: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE articles SET archive_status = 'ready', zim_main_path = ?2,
            archive_last_error = NULL, updated_at = ?3
         WHERE id = ?1",
        params![id, zim_main_path, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn set_archive_failed(conn: &Connection, id: &str, error: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE articles SET archive_status = 'failed', archive_last_error = ?2, updated_at = ?3
         WHERE id = ?1",
        params![id, error, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

/// Sets (or, with `None`, clears) a `server`-sourced article's local
/// archive cache path — shared by a fresh download and by eviction (LRU
/// sweep or manual mark-as-read), which both just null it back out.
pub fn set_archive_local_path(
    conn: &Connection,
    id: &str,
    zim_path: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE articles SET zim_path = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, zim_path, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

/// Nulls a `server`-sourced article's local archive cache path and
/// returns what it was, for the caller to unlink — or `None` if there was
/// nothing cached (already evicted), or the row is `local_legacy`, which
/// this never touches: that archive predates server-side capture and is
/// the only copy that exists anywhere, so it's never evicted.
pub fn evict_local_archive(conn: &Connection, id: &str) -> rusqlite::Result<Option<String>> {
    let zim_path: Option<String> = conn
        .query_row(
            "SELECT zim_path FROM articles WHERE id = ?1 AND archive_source = 'server'",
            params![id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    if zim_path.is_some() {
        conn.execute(
            "UPDATE articles SET zim_path = NULL, updated_at = ?2 WHERE id = ?1",
            params![id, Utc::now().to_rfc3339()],
        )?;
    }
    Ok(zim_path)
}

/// Evicts every `server`-sourced article's locally cached full archive
/// beyond the `cap` most-recently-opened, nulling their `zim_path` in the
/// same pass and returning `(id, old_zim_path)` for the caller to unlink
/// and evict from the in-memory `ZimCache`. `local_legacy` rows are never
/// selected (they're permanently local, no eviction). A `server` row
/// that's cached but was never opened (`archive_last_opened_at IS NULL`)
/// sorts as least-recently-used and is evicted first — SQLite orders
/// `NULL` before any real value in `DESC`, which is exactly the
/// eviction-first behavior wanted here, not a special case to code around.
pub fn enforce_archive_lru(conn: &Connection, cap: u32) -> rusqlite::Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT id, zim_path FROM articles
         WHERE archive_source = 'server' AND zim_path IS NOT NULL
         ORDER BY archive_last_opened_at DESC
         LIMIT -1 OFFSET ?1",
    )?;
    let evicted: Vec<(String, String)> = stmt
        .query_map(params![cap], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    for (id, _) in &evicted {
        conn.execute("UPDATE articles SET zim_path = NULL WHERE id = ?1", params![id])?;
    }
    Ok(evicted)
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
            "archive_server_url" => settings.archive_server_url = value,
            "archive_server_token" => settings.archive_server_token = value,
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
        "INSERT INTO settings (key, value) VALUES ('archive_server_url', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![settings.archive_server_url],
    )?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES ('archive_server_token', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![settings.archive_server_token],
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

    /// Inserts `output` and directly sets `archive_source`/`zim_path`/
    /// `archive_last_opened_at`, bypassing the normal capture flow — this
    /// module's tests only need to construct specific LRU-relevant states,
    /// not exercise how an article actually gets there.
    fn insert_cached_article(
        conn: &Connection,
        id: &str,
        archive_source: &str,
        opened_at: &str,
    ) {
        let output = sample_capture_output(&format!("https://example.com/{id}"));
        insert_captured_article(conn, id, None, "Direct link", "direct", &output).unwrap();
        conn.execute(
            "UPDATE articles SET archive_source = ?2, zim_path = ?3, archive_last_opened_at = ?4 WHERE id = ?1",
            params![id, archive_source, format!("archives/{id}.zim"), opened_at],
        )
        .unwrap();
    }

    #[test]
    fn enforce_archive_lru_evicts_only_the_least_recently_opened_server_rows() {
        let conn = migrated_conn();
        // Six `server` rows, oldest (art-0) to newest (art-5) by
        // `archive_last_opened_at`, plus one `local_legacy` row with an
        // even older timestamp than all of them.
        insert_cached_article(&conn, "legacy", "local_legacy", "2020-01-01T00:00:00Z");
        for i in 0..6 {
            insert_cached_article(
                &conn,
                &format!("art-{i}"),
                "server",
                &format!("2026-01-0{}T00:00:00Z", i + 1),
            );
        }

        let evicted = enforce_archive_lru(&conn, 5).expect("enforce lru");
        let evicted_ids: Vec<&str> = evicted.iter().map(|(id, _)| id.as_str()).collect();

        assert_eq!(evicted_ids, vec!["art-0"], "only the single oldest server row beyond the cap of 5 should be evicted");

        let legacy_zim_path: Option<String> = conn
            .query_row("SELECT zim_path FROM articles WHERE id = 'legacy'", [], |row| row.get(0))
            .unwrap();
        assert!(legacy_zim_path.is_some(), "local_legacy rows must never be evicted, however old");

        let evicted_row_zim_path: Option<String> = conn
            .query_row("SELECT zim_path FROM articles WHERE id = 'art-0'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(evicted_row_zim_path, None, "evicted row's zim_path must be nulled");

        let kept_row_zim_path: Option<String> = conn
            .query_row("SELECT zim_path FROM articles WHERE id = 'art-5'", [], |row| row.get(0))
            .unwrap();
        assert!(kept_row_zim_path.is_some(), "the most-recently-opened rows must stay cached");
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
        insert_captured_article(&conn, "art-1", Some(&source.id), &source.name, "rss", &output).unwrap();
        assert_eq!(get_source(&conn, &source.id).unwrap().unwrap().article_count, 1);

        let deleted = delete_article(&conn, "art-1").unwrap().expect("row existed");
        assert_eq!(deleted.zim_path, None, "no full archive is cached locally yet at insert time");
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

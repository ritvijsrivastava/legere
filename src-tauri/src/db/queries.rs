use std::collections::HashSet;

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, Row, params};
use uuid::Uuid;

use crate::capture::LocalCaptureOutput;
use crate::models::{ArticleDetail, ArticleSummary, Category, Settings, Source};

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

/// The one invariant every tag-writing path shares: lowercase, trimmed,
/// non-empty, de-duplicated (first occurrence wins, order otherwise
/// preserved). Called from every insert/update path that accepts tags
/// (RSS capture, Raindrop import, manual edits) so "tags are always
/// lowercase" holds regardless of what a caller passes in, rather than
/// relying on each call site to remember to normalize itself.
fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for tag in tags {
        let tag = tag.trim().to_lowercase();
        if !tag.is_empty() && seen.insert(tag.clone()) {
            result.push(tag);
        }
    }
    result
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

const ARTICLE_SUMMARY_COLUMNS: &str =
    "id, title, source_name, source_type, excerpt, hero_image_path,
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
/// `capture_local` was called with, since that's what its content/
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
            read_time_min, favorited,
            extraction_confident, tags, updated_at
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
            output.extraction_confident,
            tags_to_json(&normalize_tags(tags)),
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

/// Inserts an article captured by `raindrop_import::run_import`. Unlike
/// [`insert_captured_article`], `fetched_at` is backdated to `saved_at`
/// (the bookmark's own original save timestamp from the import source,
/// e.g. Raindrop's `created` column) rather than "now", so an imported
/// article's place in the library's fetched-at ordering reflects when it
/// was actually bookmarked, not when this import ran; `favorited` is
/// likewise taken directly from the import data instead of defaulting to
/// `false`. Always `source_id = NULL` — an import isn't tied to a
/// recurring source. Returns `true`/`false` on insert/duplicate exactly
/// like `insert_captured_article`.
pub fn insert_imported_article(
    conn: &Connection,
    id: &str,
    source_name: &str,
    output: &LocalCaptureOutput,
    tags: &[String],
    saved_at: &str,
    favorited: bool,
) -> rusqlite::Result<bool> {
    let now = Utc::now().to_rfc3339();
    let inserted = conn.execute(
        "INSERT OR IGNORE INTO articles (
            id, source_id, source_name, source_type, title, link, excerpt,
            content_html, hero_image_path, published_at, fetched_at,
            read_time_min, favorited,
            extraction_confident, tags, updated_at
        ) VALUES (?1, NULL, ?2, 'direct', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            id,
            source_name,
            output.title,
            output.link,
            output.excerpt,
            output.content_html,
            output.hero_image_path,
            output.published_at,
            saved_at,
            output.read_time_min,
            favorited,
            output.extraction_confident,
            tags_to_json(&normalize_tags(tags)),
            now,
        ],
    )? > 0;
    if !inserted {
        merge_tags_by_link(conn, &output.link, tags)?;
    }
    Ok(inserted)
}

/// Unions `incoming` tags into whatever's already stored on the article
/// at `link`, so re-importing a CSV whose rows now carry tags that
/// weren't there (or weren't yet applied) on a previous run still picks
/// those up on an otherwise-duplicate row instead of silently discarding
/// them. A no-op — no `UPDATE` at all — when every incoming tag (after
/// [`normalize_tags`]) is already present, so a plain re-import of an
/// unchanged CSV doesn't bump `updated_at` on every single row. Existing
/// tag order is preserved; new tags are appended in their normalized
/// order. Silently does nothing if `link` doesn't match any row (the
/// caller only reaches here when it already knows the link duplicated
/// *something*, but doesn't hold that row's id).
fn merge_tags_by_link(conn: &Connection, link: &str, incoming: &[String]) -> rusqlite::Result<()> {
    let incoming = normalize_tags(incoming);
    if incoming.is_empty() {
        return Ok(());
    }
    let Some((existing_id, existing_tags)) = conn
        .query_row(
            "SELECT id, tags FROM articles WHERE link = ?1",
            params![link],
            |row| Ok((row.get::<_, String>(0)?, parse_tags(row.get(1)?))),
        )
        .optional()?
    else {
        return Ok(());
    };
    let existing_set: HashSet<&str> = existing_tags.iter().map(String::as_str).collect();
    let mut merged = existing_tags.clone();
    for tag in &incoming {
        if !existing_set.contains(tag.as_str()) {
            merged.push(tag.clone());
        }
    }
    if merged.len() == existing_tags.len() {
        return Ok(());
    }
    conn.execute(
        "UPDATE articles SET tags = ?2, updated_at = ?3 WHERE id = ?1",
        params![existing_id, tags_to_json(&merged), Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

/// Overwrites an existing article's *readable* content in place, for
/// `recapture_article` — re-running the local capture pipeline against the
/// same id (and thus the same on-disk content/hero-image paths, which the
/// caller has already overwritten by this point). `reading_state`/
/// `favorited`/`reading_progress` are deliberately left untouched: a
/// re-capture refreshes the *content*, not the reader's relationship to
/// it. `link` is updated too — if that collides with another article's
/// `UNIQUE(link)`, this fails with a constraint error rather than
/// silently corrupting either row, which is the right outcome for a
/// manual, occasional action.
pub fn update_captured_article(
    conn: &Connection,
    id: &str,
    output: &LocalCaptureOutput,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE articles SET
            title = ?2, link = ?3, excerpt = ?4, content_html = ?5,
            hero_image_path = ?6, published_at = ?7, read_time_min = ?8,
            extraction_confident = ?9,
            updated_at = ?10
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
            output.extraction_confident,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

/// Superseded by [`list_articles_page`] for the frontend's own use (it
/// stopped scaling once a library reached hundreds/thousands of rows),
/// but kept for test fixtures (`sources::rss`, `sources::raindrop_import`)
/// that just want "everything currently in the table" without dealing
/// with pagination.
#[cfg_attr(not(test), allow(dead_code))]
pub fn list_articles(conn: &Connection) -> rusqlite::Result<Vec<ArticleSummary>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {ARTICLE_SUMMARY_COLUMNS} FROM articles ORDER BY fetched_at DESC"
    ))?;
    let rows = stmt.query_map([], article_summary_from_row)?;
    rows.collect()
}

/// Escapes `%`/`_` (SQLite `LIKE` wildcards) and the escape character
/// itself in free-text search input, so a user searching for e.g. `50%
/// done` doesn't have `%` behave as a wildcard. Paired with `ESCAPE '\'`
/// on the `LIKE` clause in [`list_articles_page`].
fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Filters/cursor for [`list_articles_page`] — borrowed rather than
/// owned since it's only ever used for the lifetime of a single query.
pub struct ArticlePageQuery<'a> {
    /// `(fetched_at, id)` of the last row on the previous page; `None`
    /// fetches the first page.
    pub cursor: Option<(&'a str, &'a str)>,
    pub limit: i64,
    pub search: Option<&'a str>,
    pub source_name: Option<&'a str>,
    /// Any-of match against the article's `tags` JSON array, via
    /// `json_each` (bundled SQLite has had the JSON functions built in,
    /// no extension needed, since 3.38 — comfortably covered by this
    /// crate's bundled version).
    pub tags: &'a [String],
    pub favorited_only: bool,
}

/// [`list_articles_page`]'s result. `next_cursor` is `Some((fetched_at,
/// id))` of the last row in `items` whenever `has_more` is true, ready to
/// pass straight back in as the next call's `ArticlePageQuery::cursor` —
/// callers never need to know `ArticleSummary` doesn't itself carry
/// `fetched_at`.
pub struct ArticlePageResult {
    pub items: Vec<ArticleSummary>,
    pub has_more: bool,
    pub next_cursor: Option<(String, String)>,
}

/// Keyset-paginated (not `OFFSET`-based, which would mean re-scanning and
/// discarding an ever-growing prefix as the user pages deeper) article
/// listing, ordered by `fetched_at DESC, id DESC` — the `id` tiebreak
/// makes the sort fully deterministic (two rows can share a `fetched_at`,
/// e.g. backdated Raindrop imports), which keyset pagination depends on
/// to never skip or repeat a row across pages. Every filter predicate is
/// applied in SQL so a page reflects the *whole* table, not just whatever
/// happens to already be loaded client-side.
///
/// `has_more` comes from fetching one extra row past `limit`, not from
/// comparing the returned count to `limit`.
pub fn list_articles_page(
    conn: &Connection,
    query: &ArticlePageQuery,
) -> rusqlite::Result<ArticlePageResult> {
    let mut sql = format!("SELECT {ARTICLE_SUMMARY_COLUMNS}, fetched_at FROM articles WHERE 1 = 1");
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if query.favorited_only {
        sql.push_str(" AND favorited = 1");
    }
    if let Some(search) = query.search {
        sql.push_str(" AND title LIKE ? ESCAPE '\\'");
        params.push(Box::new(format!("%{}%", escape_like(search))));
    }
    if let Some(source_name) = query.source_name {
        sql.push_str(" AND source_name = ?");
        params.push(Box::new(source_name.to_string()));
    }
    if !query.tags.is_empty() {
        let placeholders = vec!["?"; query.tags.len()].join(", ");
        sql.push_str(&format!(
            " AND EXISTS (SELECT 1 FROM json_each(articles.tags) WHERE json_each.value IN ({placeholders}))"
        ));
        for tag in query.tags {
            // Stored tags are always lowercase (`normalize_tags`); lowercase
            // the filter value too so a stray mixed-case caller still
            // matches instead of silently returning nothing.
            params.push(Box::new(tag.trim().to_lowercase()));
        }
    }
    if let Some((fetched_at, id)) = query.cursor {
        sql.push_str(" AND (fetched_at < ? OR (fetched_at = ? AND id < ?))");
        params.push(Box::new(fetched_at.to_string()));
        params.push(Box::new(fetched_at.to_string()));
        params.push(Box::new(id.to_string()));
    }
    sql.push_str(" ORDER BY fetched_at DESC, id DESC LIMIT ?");
    // Fetch one row past what was asked for, purely to answer `has_more`
    // without a second round trip.
    params.push(Box::new(query.limit + 1));

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut rows: Vec<(ArticleSummary, String)> = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok((
                article_summary_from_row(row)?,
                row.get::<_, String>("fetched_at")?,
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;

    let has_more = rows.len() as i64 > query.limit;
    rows.truncate(query.limit.max(0) as usize);
    let next_cursor = has_more
        .then(|| {
            rows.last()
                .map(|(item, fetched_at)| (fetched_at.clone(), item.id.clone()))
        })
        .flatten();
    let items = rows.into_iter().map(|(item, _)| item).collect();
    Ok(ArticlePageResult {
        items,
        has_more,
        next_cursor,
    })
}

pub fn count_all_articles(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row("SELECT COUNT(*) FROM articles", [], |row| row.get(0))
}

pub fn count_unread(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM articles WHERE reading_state = 'unread'",
        [],
        |row| row.get(0),
    )
}

pub fn count_favorited(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM articles WHERE favorited = 1",
        [],
        |row| row.get(0),
    )
}

/// Distinct `source_name`s with their article counts, for the sidebar's
/// Superseded by the real, user-managed `categories` table (`fetch_categories`
/// and friends, below) — kept only until the sidebar/library filters are
/// cut over to `category_id` (see `PLAN.md`'s categories addendum). Groups
/// by `source_name`, which was never a real category, just this table's
/// placeholder for one before the `categories` table existed.
///
/// Categories section and the mobile category-chip row — the SQL
/// equivalent of the old `deriveCategories(articlesStore.items)`, which
/// stopped being viable once the frontend no longer holds every article
/// in memory.
pub fn list_categories(conn: &Connection) -> rusqlite::Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT source_name, COUNT(*) FROM articles GROUP BY source_name ORDER BY source_name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    rows.collect()
}

/// Distinct tags (from every article's `tags` JSON array) with counts,
/// for the sidebar's Tags section — the SQL equivalent of the old
/// `deriveTags(articlesStore.items)`.
pub fn list_tags(conn: &Connection) -> rusqlite::Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT value, COUNT(*) FROM articles, json_each(articles.tags)
         GROUP BY value ORDER BY value COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    rows.collect()
}

fn category_from_row(row: &Row) -> rusqlite::Result<Category> {
    Ok(Category {
        id: row.get("id")?,
        name: row.get("name")?,
        article_count: row.get("article_count")?,
    })
}

/// Every category ("folder") with its live article count, including ones
/// with zero articles — unlike the old `source_name`-grouped
/// [`list_categories`], a category is its own row (see `db::schema`'s
/// `V9`) so it doesn't disappear from this list just because its last
/// article was reassigned elsewhere. Ordered case-insensitively by name,
/// same convention as [`list_tags`].
pub fn fetch_categories(conn: &Connection) -> rusqlite::Result<Vec<Category>> {
    let mut stmt = conn.prepare(
        "SELECT c.id AS id, c.name AS name, COUNT(a.id) AS article_count
         FROM categories c
         LEFT JOIN articles a ON a.category_id = c.id
         GROUP BY c.id
         ORDER BY c.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], category_from_row)?;
    rows.collect()
}

pub fn get_category(conn: &Connection, id: &str) -> rusqlite::Result<Option<Category>> {
    conn.query_row(
        "SELECT c.id AS id, c.name AS name, COUNT(a.id) AS article_count
         FROM categories c
         LEFT JOIN articles a ON a.category_id = c.id
         WHERE c.id = ?1
         GROUP BY c.id",
        params![id],
        category_from_row,
    )
    .optional()
}

/// Creates a new, empty category named `name` (trimmed). `name`'s
/// uniqueness is enforced case-insensitively by `idx_categories_name`
/// (`COLLATE NOCASE`) — a collision surfaces as a plain
/// `rusqlite::Error::SqliteFailure` with `ConstraintViolation`, which
/// `commands::categories::create_category` matches on to return a clean
/// "already exists" error instead of a raw SQLite message.
pub fn create_category(conn: &Connection, name: &str) -> rusqlite::Result<Category> {
    let name = name.trim();
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO categories (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
        params![id, name, now],
    )?;
    Ok(Category {
        id,
        name: name.to_string(),
        article_count: 0,
    })
}

/// Finds an existing category by case-insensitive name, or creates one —
/// the Raindrop-import path's folder-to-category resolution (a folder
/// name reused across import runs, or one that happens to already match
/// a manually-created category, should land in the same category rather
/// than erroring or duplicating). Unlike [`create_category`], never fails
/// on a name collision.
pub fn find_or_create_category(conn: &Connection, name: &str) -> rusqlite::Result<Category> {
    let name = name.trim();
    if let Some(existing) = conn
        .query_row(
            "SELECT id, name, 0 AS article_count FROM categories WHERE name = ?1 COLLATE NOCASE",
            params![name],
            category_from_row,
        )
        .optional()?
    {
        return Ok(existing);
    }
    create_category(conn, name)
}

/// Renames an existing category (still subject to the same case-
/// insensitive uniqueness as [`create_category`]). Returns `None` if `id`
/// doesn't match any row; the rest of this module's "missing id" convention
/// (see e.g. [`set_article_tags`]).
pub fn rename_category(
    conn: &Connection,
    id: &str,
    name: &str,
) -> rusqlite::Result<Option<Category>> {
    let name = name.trim();
    let changed = conn.execute(
        "UPDATE categories SET name = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, name, Utc::now().to_rfc3339()],
    )?;
    if changed == 0 {
        return Ok(None);
    }
    get_category(conn, id)
}

/// Deletes a category outright. Its articles are **not** deleted —
/// `articles.category_id`'s `ON DELETE SET NULL` (see `db::schema`'s
/// `V9`) un-categorizes them instead, same as the "move to an empty
/// collection" choice the Raindrop-import conflict UI offers. Returns
/// `true` if a row was actually deleted.
pub fn delete_category(conn: &Connection, id: &str) -> rusqlite::Result<bool> {
    let changed = conn.execute("DELETE FROM categories WHERE id = ?1", params![id])?;
    Ok(changed > 0)
}

/// Sets (or clears, for `category_id = None`) a single article's
/// category. Returns `false` if `id` doesn't match any article; a
/// `category_id` that doesn't match any category is rejected by the
/// `REFERENCES categories(id)` foreign key instead of silently
/// succeeding (surfaces as a `rusqlite::Error`).
pub fn set_article_category(
    conn: &Connection,
    id: &str,
    category_id: Option<&str>,
) -> rusqlite::Result<bool> {
    let changed = conn.execute(
        "UPDATE articles SET category_id = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, category_id, Utc::now().to_rfc3339()],
    )?;
    Ok(changed > 0)
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

/// Whether `id` names a real, currently-stored article row — the
/// `legere-content://` protocol handler's traversal guard (see
/// `content_server::serve`): only an id that's actually a stored
/// article's row resolves to a real file, so an arbitrary/forged id in a
/// request can't reach any other file under `content/`.
pub fn article_exists(conn: &Connection, id: &str) -> rusqlite::Result<bool> {
    conn.query_row("SELECT 1 FROM articles WHERE id = ?1", params![id], |_| {
        Ok(())
    })
    .optional()
    .map(|r| r.is_some())
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

/// On-disk file/directory paths an about-to-be-deleted article owns, so
/// the caller can remove them after the row itself is gone.
pub struct DeletedArticleFiles {
    pub hero_image_path: Option<String>,
}

/// Deletes an article and decrements its source's `article_count` (floored
/// at 0). Returns `None` if `id` didn't match any row — deleting an
/// already-gone article is treated as a no-op success by the caller, not
/// an error. `remove_source` deliberately does *not* cascade to articles
/// (`ON DELETE SET NULL` — read-later semantics: removing a feed doesn't
/// discard what you already saved from it), so this is the only path that
/// ever deletes an article row.
///
/// The caller is also responsible for removing this id's `content/<id>/`
/// directory — deterministic by id, so it isn't tracked in the returned
/// struct the way `hero_image_path` (which varies) needs to be.
pub fn delete_article(
    conn: &Connection,
    id: &str,
) -> rusqlite::Result<Option<DeletedArticleFiles>> {
    let row = conn
        .query_row(
            "SELECT source_id, hero_image_path FROM articles WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>("source_id")?,
                    row.get::<_, Option<String>>("hero_image_path")?,
                ))
            },
        )
        .optional()?;

    let Some((source_id, hero_image_path)) = row else {
        return Ok(None);
    };

    conn.execute("DELETE FROM articles WHERE id = ?1", params![id])?;

    if let Some(sid) = source_id {
        conn.execute(
            "UPDATE sources SET article_count = MAX(0, article_count - 1), updated_at = ?2 WHERE id = ?1",
            params![sid, Utc::now().to_rfc3339()],
        )?;
    }

    Ok(Some(DeletedArticleFiles { hero_image_path }))
}

/// Deletes every article row and resets every source's `article_count`
/// to 0. Used by the settings "delete all articles" action — a
/// deliberately blunt, irreversible bulk op, unlike `delete_article`'s
/// per-row path. The caller is responsible for removing the on-disk
/// `content/` and `media/` directories wholesale, since there's no per-
/// article file list worth collecting when everything is going away.
pub fn delete_all_articles(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM articles", [])?;
    conn.execute(
        "UPDATE sources SET article_count = 0, updated_at = ?1",
        params![Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

/// Every live article's id, plus every `hero_image_path` currently
/// referenced — the startup orphan sweep (`gc::sweep_orphaned_files`)
/// diffs this against what's actually on disk under `content/`/`media/`:
/// a `content/<id>/` directory survives only if `<id>` is in
/// `live_article_ids`, a flat `media/` file only if it's in
/// `hero_image_paths`.
pub struct ReferencedFiles {
    pub live_article_ids: HashSet<String>,
    pub hero_image_paths: HashSet<String>,
}

pub fn list_referenced_files(conn: &Connection) -> rusqlite::Result<ReferencedFiles> {
    let mut stmt = conn.prepare("SELECT id, hero_image_path FROM articles")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    })?;
    let mut live_article_ids = HashSet::new();
    let mut hero_image_paths = HashSet::new();
    for row in rows {
        let (id, hero_image_path) = row?;
        live_article_ids.insert(id);
        if let Some(path) = hero_image_path {
            hero_image_paths.insert(path);
        }
    }
    Ok(ReferencedFiles {
        live_article_ids,
        hero_image_paths,
    })
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

/// Replaces an article's whole tag set (not a single add/remove) — the
/// manual tag editor in the reader always submits the full list it's
/// currently showing, same shape as [`normalize_tags`] expects. Returns
/// the normalized tags actually stored, or `None` if `id` doesn't match
/// any row (checked via `changes()`, since an UPDATE against a missing id
/// silently affects zero rows rather than erroring).
pub fn set_article_tags(
    conn: &Connection,
    id: &str,
    tags: &[String],
) -> rusqlite::Result<Option<Vec<String>>> {
    let normalized = normalize_tags(tags);
    let changed = conn.execute(
        "UPDATE articles SET tags = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, tags_to_json(&normalized), Utc::now().to_rfc3339()],
    )?;
    Ok((changed > 0).then_some(normalized))
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

pub fn insert_rss_source(
    conn: &Connection,
    name: &str,
    feed_url: &str,
) -> rusqlite::Result<Source> {
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
            extraction_confident: true,
        }
    }

    #[test]
    fn toggle_source_pause_recovers_a_source_from_error_status() {
        let conn = migrated_conn();
        let source = insert_rss_source(&conn, "Feed", "https://example.com/feed.xml").unwrap();
        mark_source_error(&conn, &source.id, "boom").unwrap();
        assert_eq!(
            get_source(&conn, &source.id).unwrap().unwrap().status,
            "error"
        );

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
        insert_captured_article(
            &conn,
            "art-1",
            Some(&source.id),
            &source.name,
            "rss",
            &output,
            &[],
        )
        .unwrap();
        assert_eq!(
            get_source(&conn, &source.id)
                .unwrap()
                .unwrap()
                .article_count,
            1
        );

        let deleted = delete_article(&conn, "art-1")
            .unwrap()
            .expect("row existed");
        assert_eq!(deleted.hero_image_path, None);

        assert!(get_article(&conn, "art-1").unwrap().is_none());
        assert_eq!(
            get_source(&conn, &source.id)
                .unwrap()
                .unwrap()
                .article_count,
            0
        );
    }

    #[test]
    fn insert_captured_article_normalizes_tags_to_lowercase_trimmed_deduped() {
        let conn = migrated_conn();
        let output = sample_capture_output("https://example.com/tagged");
        let tags = vec![
            "Rust".to_string(),
            " rust ".to_string(),
            "WebDev".to_string(),
            "".to_string(),
        ];
        insert_captured_article(
            &conn,
            "art-tagged",
            None,
            "Direct link",
            "direct",
            &output,
            &tags,
        )
        .unwrap();

        let article = get_article(&conn, "art-tagged").unwrap().unwrap();
        assert_eq!(article.tags, vec!["rust".to_string(), "webdev".to_string()]);
    }

    #[test]
    fn set_article_tags_replaces_and_normalizes_the_tag_set() {
        let conn = migrated_conn();
        let output = sample_capture_output("https://example.com/retagged");
        insert_captured_article(
            &conn,
            "art-retag",
            None,
            "Direct link",
            "direct",
            &output,
            &["old-tag".to_string()],
        )
        .unwrap();

        let updated = set_article_tags(
            &conn,
            "art-retag",
            &[
                "New Tag".to_string(),
                "new tag".to_string(),
                " ".to_string(),
            ],
        )
        .unwrap()
        .expect("row existed");
        assert_eq!(updated, vec!["new tag".to_string()]);

        let article = get_article(&conn, "art-retag").unwrap().unwrap();
        assert_eq!(article.tags, vec!["new tag".to_string()]);
    }

    #[test]
    fn set_article_tags_returns_none_for_a_missing_id() {
        let conn = migrated_conn();
        assert_eq!(
            set_article_tags(&conn, "does-not-exist", &[]).unwrap(),
            None
        );
    }

    #[test]
    fn create_category_and_fetch_categories_reflect_live_article_counts() {
        let conn = migrated_conn();
        let recipes = create_category(&conn, "Recipes").unwrap();
        create_category(&conn, "Travel").unwrap();

        let output = sample_capture_output("https://example.com/recipe-1");
        insert_captured_article(&conn, "art-1", None, "Direct link", "direct", &output, &[])
            .unwrap();
        set_article_category(&conn, "art-1", Some(&recipes.id)).unwrap();

        let categories = fetch_categories(&conn).unwrap();
        assert_eq!(
            categories
                .iter()
                .map(|c| (c.name.as_str(), c.article_count))
                .collect::<Vec<_>>(),
            vec![("Recipes", 1), ("Travel", 0)],
            "a category with zero articles must still be listed"
        );
    }

    #[test]
    fn create_category_rejects_a_case_insensitive_duplicate_name() {
        let conn = migrated_conn();
        create_category(&conn, "Recipes").unwrap();
        let err = create_category(&conn, "recipes").unwrap_err();
        assert!(matches!(
            err,
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: rusqlite::ErrorCode::ConstraintViolation,
                    ..
                },
                _
            )
        ));
    }

    #[test]
    fn find_or_create_category_reuses_an_existing_case_insensitive_match() {
        let conn = migrated_conn();
        let first = create_category(&conn, "Recipes").unwrap();

        let found = find_or_create_category(&conn, "recipes").unwrap();
        assert_eq!(
            found.id, first.id,
            "must reuse the existing row, not create a new one"
        );
        assert_eq!(fetch_categories(&conn).unwrap().len(), 1);

        let created = find_or_create_category(&conn, "Travel").unwrap();
        assert_ne!(created.id, first.id);
        assert_eq!(fetch_categories(&conn).unwrap().len(), 2);
    }

    #[test]
    fn rename_category_returns_none_for_a_missing_id() {
        let conn = migrated_conn();
        assert_eq!(
            rename_category(&conn, "does-not-exist", "New Name").unwrap(),
            None
        );
    }

    #[test]
    fn rename_category_updates_the_name_in_place() {
        let conn = migrated_conn();
        let category = create_category(&conn, "Recipes").unwrap();

        let renamed = rename_category(&conn, &category.id, "Cooking")
            .unwrap()
            .expect("row existed");
        assert_eq!(renamed.name, "Cooking");
        assert_eq!(renamed.id, category.id);
    }

    #[test]
    fn delete_category_orphans_its_articles_instead_of_deleting_them() {
        let conn = migrated_conn();
        let category = create_category(&conn, "Recipes").unwrap();
        let output = sample_capture_output("https://example.com/recipe-2");
        insert_captured_article(&conn, "art-1", None, "Direct link", "direct", &output, &[])
            .unwrap();
        set_article_category(&conn, "art-1", Some(&category.id)).unwrap();

        let deleted = delete_category(&conn, &category.id).unwrap();
        assert!(deleted);
        assert!(get_category(&conn, &category.id).unwrap().is_none());

        let article = get_article(&conn, "art-1").unwrap().unwrap();
        assert!(
            article.id == "art-1",
            "article must survive its category's deletion"
        );
    }

    #[test]
    fn delete_category_returns_false_for_a_missing_id() {
        let conn = migrated_conn();
        assert!(!delete_category(&conn, "does-not-exist").unwrap());
    }

    #[test]
    fn set_article_category_returns_false_for_a_missing_article() {
        let conn = migrated_conn();
        let category = create_category(&conn, "Recipes").unwrap();
        assert!(!set_article_category(&conn, "does-not-exist", Some(&category.id)).unwrap());
    }

    #[test]
    fn set_article_category_rejects_an_unknown_category_id() {
        let conn = migrated_conn();
        let output = sample_capture_output("https://example.com/recipe-3");
        insert_captured_article(&conn, "art-1", None, "Direct link", "direct", &output, &[])
            .unwrap();

        let result = set_article_category(&conn, "art-1", Some("does-not-exist"));
        assert!(
            result.is_err(),
            "the foreign key should reject a category_id that doesn't exist"
        );
    }

    #[test]
    fn set_article_category_can_clear_back_to_uncategorized() {
        let conn = migrated_conn();
        let category = create_category(&conn, "Recipes").unwrap();
        let output = sample_capture_output("https://example.com/recipe-4");
        insert_captured_article(&conn, "art-1", None, "Direct link", "direct", &output, &[])
            .unwrap();
        set_article_category(&conn, "art-1", Some(&category.id)).unwrap();

        set_article_category(&conn, "art-1", None).unwrap();

        let categories = fetch_categories(&conn).unwrap();
        assert_eq!(categories[0].article_count, 0);
    }

    #[test]
    fn insert_imported_article_backdates_fetched_at_and_sets_favorited_and_tags() {
        let conn = migrated_conn();
        let output = sample_capture_output("https://example.com/imported");
        let tags = vec!["pdf".to_string(), "file-system".to_string()];
        let inserted = insert_imported_article(
            &conn,
            "art-imported",
            "Raindrop import",
            &output,
            &tags,
            "2024-10-02T16:40:49.533Z",
            true,
        )
        .unwrap();
        assert!(inserted);

        let article = get_article(&conn, "art-imported").unwrap().unwrap();
        assert!(article.favorited);
        assert_eq!(article.tags, tags);
        assert_eq!(article.source_type, "direct");

        let fetched_at: String = conn
            .query_row(
                "SELECT fetched_at FROM articles WHERE id = ?1",
                ["art-imported"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(fetched_at, "2024-10-02T16:40:49.533Z");
    }

    #[test]
    fn insert_imported_article_is_a_no_op_for_a_duplicate_link() {
        let conn = migrated_conn();
        let output = sample_capture_output("https://example.com/dup-import");
        let first = insert_imported_article(
            &conn,
            "art-1",
            "Raindrop import",
            &output,
            &[],
            "2024-01-01T00:00:00Z",
            false,
        )
        .unwrap();
        assert!(first);

        let second = insert_imported_article(
            &conn,
            "art-2",
            "Raindrop import",
            &output,
            &[],
            "2024-01-02T00:00:00Z",
            false,
        )
        .unwrap();
        assert!(
            !second,
            "UNIQUE(link) should silently absorb the duplicate insert"
        );
        assert!(get_article(&conn, "art-2").unwrap().is_none());
    }

    #[test]
    fn reimporting_a_duplicate_link_merges_new_tags_into_the_existing_article() {
        let conn = migrated_conn();
        let output = sample_capture_output("https://example.com/retag-import");
        insert_imported_article(
            &conn,
            "art-1",
            "Raindrop import",
            &output,
            &["pdf".to_string()],
            "2024-01-01T00:00:00Z",
            false,
        )
        .unwrap();

        // Re-"importing" the same link with an overlapping-plus-new tag
        // set: the duplicate insert is absorbed as before, but the
        // *existing* row should pick up the new tag without losing the
        // one it already had.
        let second = insert_imported_article(
            &conn,
            "art-2",
            "Raindrop import",
            &output,
            &["PDF".to_string(), "file-system".to_string()],
            "2024-01-02T00:00:00Z",
            false,
        )
        .unwrap();
        assert!(!second);

        let article = get_article(&conn, "art-1").unwrap().unwrap();
        assert_eq!(
            article.tags,
            vec!["pdf".to_string(), "file-system".to_string()]
        );
    }

    #[test]
    fn reimporting_a_duplicate_link_with_no_new_tags_is_a_true_no_op() {
        let conn = migrated_conn();
        let output = sample_capture_output("https://example.com/retag-noop");
        insert_imported_article(
            &conn,
            "art-1",
            "Raindrop import",
            &output,
            &["pdf".to_string()],
            "2024-01-01T00:00:00Z",
            false,
        )
        .unwrap();
        let updated_at_before: String = conn
            .query_row(
                "SELECT updated_at FROM articles WHERE id = 'art-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        insert_imported_article(
            &conn,
            "art-2",
            "Raindrop import",
            &output,
            &["pdf".to_string()],
            "2024-01-02T00:00:00Z",
            false,
        )
        .unwrap();

        let updated_at_after: String = conn
            .query_row(
                "SELECT updated_at FROM articles WHERE id = 'art-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            updated_at_before, updated_at_after,
            "no new tags means no UPDATE at all"
        );
    }

    #[test]
    fn delete_article_is_a_no_op_for_an_unknown_id() {
        let conn = migrated_conn();
        assert!(delete_article(&conn, "does-not-exist").unwrap().is_none());
    }

    #[test]
    fn delete_all_articles_clears_rows_and_resets_source_counts() {
        let conn = migrated_conn();
        let source = insert_rss_source(&conn, "Feed", "https://example.com/feed.xml").unwrap();
        for i in 0..3 {
            let output = sample_capture_output(&format!("https://example.com/article-{i}"));
            insert_captured_article(
                &conn,
                &format!("art-{i}"),
                Some(&source.id),
                &source.name,
                "rss",
                &output,
                &[],
            )
            .unwrap();
        }
        assert_eq!(
            get_source(&conn, &source.id)
                .unwrap()
                .unwrap()
                .article_count,
            3
        );

        delete_all_articles(&conn).unwrap();

        assert_eq!(count_all_articles(&conn).unwrap(), 0);
        assert_eq!(
            get_source(&conn, &source.id)
                .unwrap()
                .unwrap()
                .article_count,
            0
        );
        // The source row itself survives — this is read-later cleanup, not
        // source removal.
        assert!(get_source(&conn, &source.id).unwrap().is_some());
    }

    #[test]
    fn set_source_name_if_default_only_replaces_the_placeholder_once() {
        let conn = migrated_conn();
        let feed_url = "https://example.com/feed.xml";
        let source = insert_rss_source(&conn, feed_url, feed_url).unwrap();
        assert_eq!(source.name, feed_url);

        set_source_name_if_default(&conn, &source.id, "Real Feed Title", feed_url).unwrap();
        assert_eq!(
            get_source(&conn, &source.id).unwrap().unwrap().name,
            "Real Feed Title"
        );

        // A later call — even with a *different* title — must not
        // overwrite a name that's no longer the placeholder (this is also
        // what protects a future user-set custom name).
        set_source_name_if_default(&conn, &source.id, "Some Other Title", feed_url).unwrap();
        assert_eq!(
            get_source(&conn, &source.id).unwrap().unwrap().name,
            "Real Feed Title"
        );
    }

    fn titled_output(link: &str, title: &str) -> LocalCaptureOutput {
        LocalCaptureOutput {
            title: title.to_string(),
            ..sample_capture_output(link)
        }
    }

    #[test]
    fn list_articles_page_orders_by_fetched_at_desc_then_id_desc_and_paginates_without_gaps_or_dupes()
     {
        let conn = migrated_conn();
        // "c", "b", "a" deliberately share a `fetched_at` to exercise the
        // `id DESC` tiebreak that keyset pagination's determinism depends
        // on; "z-newest" is strictly newer and must sort first regardless.
        for (id, fetched_at) in [
            ("a", "2024-01-01T00:00:00Z"),
            ("b", "2024-01-01T00:00:00Z"),
            ("c", "2024-01-01T00:00:00Z"),
            ("z-newest", "2024-02-01T00:00:00Z"),
        ] {
            insert_imported_article(
                &conn,
                id,
                "Feed",
                &sample_capture_output(&format!("https://example.com/{id}")),
                &[],
                fetched_at,
                false,
            )
            .unwrap();
        }

        let mut seen = Vec::new();
        let mut cursor_owned: Option<(String, String)> = None;
        loop {
            let cursor = cursor_owned.as_ref().map(|(f, i)| (f.as_str(), i.as_str()));
            let result = list_articles_page(
                &conn,
                &ArticlePageQuery {
                    cursor,
                    limit: 2,
                    search: None,
                    source_name: None,
                    tags: &[],
                    favorited_only: false,
                },
            )
            .unwrap();
            assert!(result.items.len() <= 2);
            seen.extend(result.items.iter().map(|a| a.id.clone()));
            if !result.has_more {
                break;
            }
            let last = result.items.last().unwrap();
            // The cursor is opaque to `ArticlePageQuery` itself (it just
            // takes fetched_at/id strings) — re-fetch them the same way a
            // real caller would, from the row just returned.
            let fetched_at: String = conn
                .query_row(
                    "SELECT fetched_at FROM articles WHERE id = ?1",
                    [&last.id],
                    |r| r.get(0),
                )
                .unwrap();
            cursor_owned = Some((fetched_at, last.id.clone()));
            if seen.len() > 10 {
                panic!("pagination did not terminate");
            }
        }

        assert_eq!(seen, vec!["z-newest", "c", "b", "a"]);
    }

    #[test]
    fn list_articles_page_filters_by_search_source_name_tags_and_favorited() {
        let conn = migrated_conn();
        insert_imported_article(
            &conn,
            "rust-1",
            "Rust Blog",
            &titled_output("https://example.com/rust-1", "Understanding Ownership"),
            &["rust".to_string(), "systems".to_string()],
            "2024-01-01T00:00:00Z",
            true,
        )
        .unwrap();
        insert_imported_article(
            &conn,
            "js-1",
            "JS Weekly",
            &titled_output("https://example.com/js-1", "Async Await Patterns"),
            &["javascript".to_string()],
            "2024-01-02T00:00:00Z",
            false,
        )
        .unwrap();

        let run = |q: &ArticlePageQuery| list_articles_page(&conn, q).unwrap().items;

        let by_search = run(&ArticlePageQuery {
            cursor: None,
            limit: 10,
            search: Some("ownership"),
            source_name: None,
            tags: &[],
            favorited_only: false,
        });
        assert_eq!(
            by_search.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
            vec!["rust-1"]
        );

        let by_source = run(&ArticlePageQuery {
            cursor: None,
            limit: 10,
            search: None,
            source_name: Some("JS Weekly"),
            tags: &[],
            favorited_only: false,
        });
        assert_eq!(
            by_source.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
            vec!["js-1"]
        );

        let by_tag = run(&ArticlePageQuery {
            cursor: None,
            limit: 10,
            search: None,
            source_name: None,
            tags: &["systems".to_string()],
            favorited_only: false,
        });
        assert_eq!(
            by_tag.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
            vec!["rust-1"]
        );

        let favorited = run(&ArticlePageQuery {
            cursor: None,
            limit: 10,
            search: None,
            source_name: None,
            tags: &[],
            favorited_only: true,
        });
        assert_eq!(
            favorited.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
            vec!["rust-1"]
        );
    }

    #[test]
    fn list_articles_page_search_is_case_insensitive_and_escapes_like_wildcards() {
        let conn = migrated_conn();
        insert_imported_article(
            &conn,
            "pct",
            "Feed",
            &titled_output("https://example.com/pct", "Batteries at 50% capacity"),
            &[],
            "2024-01-01T00:00:00Z",
            false,
        )
        .unwrap();

        let run = |search: &str| {
            list_articles_page(
                &conn,
                &ArticlePageQuery {
                    cursor: None,
                    limit: 10,
                    search: Some(search),
                    source_name: None,
                    tags: &[],
                    favorited_only: false,
                },
            )
            .unwrap()
            .items
        };

        assert_eq!(
            run("BATTERIES").len(),
            1,
            "search should be case-insensitive"
        );
        assert_eq!(
            run("50% capacity").len(),
            1,
            "literal % in the query should not act as a wildcard"
        );
        assert_eq!(
            run("50x capacity").len(),
            0,
            "an unescaped % would have made this an unrelated match"
        );
    }

    #[test]
    fn count_and_list_aggregates_reflect_the_table() {
        let conn = migrated_conn();
        insert_imported_article(
            &conn,
            "a1",
            "Feed A",
            &titled_output("https://example.com/a1", "First"),
            &["tag-x".to_string()],
            "2024-01-01T00:00:00Z",
            true,
        )
        .unwrap();
        insert_imported_article(
            &conn,
            "a2",
            "Feed B",
            &titled_output("https://example.com/a2", "Second"),
            &["tag-x".to_string(), "tag-y".to_string()],
            "2024-01-02T00:00:00Z",
            false,
        )
        .unwrap();
        transition_to_read(&conn, "a2").unwrap();

        assert_eq!(count_all_articles(&conn).unwrap(), 2);
        assert_eq!(count_favorited(&conn).unwrap(), 1);
        // "a1" defaults to `unread` (never opened); "a2" was just
        // transitioned to `read`.
        assert_eq!(count_unread(&conn).unwrap(), 1);

        assert_eq!(
            list_categories(&conn).unwrap(),
            vec![("Feed A".to_string(), 1), ("Feed B".to_string(), 1)]
        );
        assert_eq!(
            list_tags(&conn).unwrap(),
            vec![("tag-x".to_string(), 2), ("tag-y".to_string(), 1)]
        );
    }
}

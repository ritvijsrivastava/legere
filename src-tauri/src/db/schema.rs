use rusqlite::Connection;
use rusqlite_migration::{HookResult, M, Migrations};

const V1: &str = "
CREATE TABLE sources (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    type          TEXT NOT NULL CHECK (type IN ('rss','mail','direct')),
    feed_url      TEXT,
    status        TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','paused','error')),
    last_error    TEXT,
    article_count INTEGER NOT NULL DEFAULT 0,
    last_synced_at TEXT,
    created_at    TEXT NOT NULL
);

CREATE TABLE articles (
    id              TEXT PRIMARY KEY,
    source_id       TEXT REFERENCES sources(id) ON DELETE SET NULL,
    source_name     TEXT NOT NULL,
    source_type     TEXT NOT NULL CHECK (source_type IN ('rss','mail','direct')),
    title           TEXT NOT NULL,
    link            TEXT NOT NULL,
    excerpt         TEXT NOT NULL,
    content_html    TEXT NOT NULL,
    hero_image_path TEXT,
    published_at    TEXT,
    fetched_at      TEXT NOT NULL,
    read_time_min   INTEGER NOT NULL DEFAULT 1,
    unread          INTEGER NOT NULL DEFAULT 1,
    favorited       INTEGER NOT NULL DEFAULT 0,
    zim_path        TEXT NOT NULL
);
CREATE INDEX idx_articles_source_id ON articles(source_id);
CREATE INDEX idx_articles_fetched_at ON articles(fetched_at DESC);
CREATE INDEX idx_articles_unread ON articles(unread);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
INSERT INTO settings (key, value) VALUES
    ('default_font_size', 'medium'),
    ('default_library_view', 'cards'),
    ('autosync', 'true');
";

// SQLite can't ALTER a CHECK constraint, so both tables are recreated
// under a `_v2` name, repopulated, and swapped in. Foreign key
// enforcement is toggled off around the whole migration by `migrate`
// below (never inside migration SQL itself — see this crate's own docs:
// `PRAGMA foreign_keys` is a no-op inside the transaction each migration
// already runs in), so `DROP TABLE sources` here can't trigger an
// implicit `ON DELETE SET NULL` cascade against the not-yet-recreated
// `articles` table.
//
// Both tables' `type`/`source_type` CHECK drops `mail`: the mail source
// was always UI-only (Add button disabled) and the backend command
// layer rejected it outright, so no `mail` *source* row can exist; any
// `mail` *article* (there shouldn't be any, for the same reason) is
// remapped to `direct` rather than dropped, since `direct` is the real
// non-recurring-source article type. `sources.type` narrows to `rss`
// only — `direct`-type sources were also never actually created
// (direct-link articles have `source_id = NULL` and no `sources` row).
//
// `articles` also gains a `UNIQUE(link)` index (this is what turns the
// existing racy `article_link_exists` pre-check into an enforced
// invariant — see `db/queries.rs::insert_captured_article`), so existing
// duplicate-link rows are collapsed onto their newest capture first; this
// is a personal, two-commit-old database, so losing older duplicates here
// is accepted rather than engineered around.
const V2: &str = "
CREATE TABLE sources_v2 (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    type          TEXT NOT NULL CHECK (type IN ('rss')),
    feed_url      TEXT,
    status        TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','paused','error')),
    last_error    TEXT,
    article_count INTEGER NOT NULL DEFAULT 0,
    last_synced_at TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);
INSERT INTO sources_v2 (
    id, name, type, feed_url, status, last_error, article_count,
    last_synced_at, created_at, updated_at
)
SELECT id, name, type, feed_url, status, last_error, article_count,
       last_synced_at, created_at, created_at
FROM sources WHERE type = 'rss';
DROP TABLE sources;
ALTER TABLE sources_v2 RENAME TO sources;

CREATE TABLE articles_v2 (
    id                   TEXT PRIMARY KEY,
    source_id            TEXT REFERENCES sources(id) ON DELETE SET NULL,
    source_name          TEXT NOT NULL,
    source_type          TEXT NOT NULL CHECK (source_type IN ('rss','direct')),
    title                TEXT NOT NULL,
    link                 TEXT NOT NULL,
    excerpt              TEXT NOT NULL,
    content_html         TEXT NOT NULL,
    hero_image_path      TEXT,
    published_at         TEXT,
    fetched_at           TEXT NOT NULL,
    read_time_min        INTEGER NOT NULL DEFAULT 1,
    unread               INTEGER NOT NULL DEFAULT 1,
    favorited            INTEGER NOT NULL DEFAULT 0,
    zim_path             TEXT NOT NULL,
    zim_main_path        TEXT NOT NULL DEFAULT 'index.html',
    extraction_confident INTEGER NOT NULL DEFAULT 1,
    reading_progress     REAL NOT NULL DEFAULT 0,
    updated_at           TEXT NOT NULL
);
INSERT INTO articles_v2 (
    id, source_id, source_name, source_type, title, link, excerpt,
    content_html, hero_image_path, published_at, fetched_at,
    read_time_min, unread, favorited, zim_path, zim_main_path,
    extraction_confident, reading_progress, updated_at
)
SELECT
    a.id, a.source_id, a.source_name,
    CASE a.source_type WHEN 'mail' THEN 'direct' ELSE a.source_type END,
    a.title, a.link, a.excerpt, a.content_html, a.hero_image_path,
    a.published_at, a.fetched_at, a.read_time_min, a.unread, a.favorited,
    a.zim_path, 'index.html', 1, 0, a.fetched_at
FROM articles a
WHERE a.rowid = (
    SELECT b.rowid FROM articles b
    WHERE b.link = a.link
    ORDER BY b.fetched_at DESC, b.rowid DESC
    LIMIT 1
);
DROP TABLE articles;
ALTER TABLE articles_v2 RENAME TO articles;

CREATE INDEX idx_articles_source_id ON articles(source_id);
CREATE INDEX idx_articles_fetched_at ON articles(fetched_at DESC);
CREATE INDEX idx_articles_unread ON articles(unread);
CREATE UNIQUE INDEX idx_articles_link ON articles(link);

INSERT INTO settings (key, value)
SELECT 'reader_font_size', CASE value
    WHEN 'small' THEN '16'
    WHEN 'large' THEN '21'
    ELSE '19'
END
FROM settings WHERE key = 'default_font_size'
ON CONFLICT(key) DO NOTHING;
";

// Introduces server-side archiving: a real `unread`/`reading`/`read` state
// machine (replacing the old `unread` boolean, which had no "finished"
// signal) and the bookkeeping needed to cache a full-page ZIM archive
// locally only while an article is being read, while a small "content
// zim" holding just the readable view's own images is never evicted.
//
// `zim_path`/`zim_main_path` are repurposed rather than replaced: both
// become nullable, and `zim_path` becomes "the full archive's *current
// local cache path*, or NULL if not cached right now" for every row (not
// just newly captured ones) — reusing the same column avoids a second
// column that has to be kept in sync with it. `zim_main_path` stays
// populated independently of whether the bytes are cached locally (it's
// metadata the server reports once capture finishes), so re-downloading
// an evicted archive never has to re-fetch status first.
//
// Migrated (pre-V3) rows get `archive_source = 'local_legacy'`: their one
// full archive already exists locally and predates server-side capture
// entirely, so the LRU sweep (which only ever considers
// `archive_source = 'server'` rows) never evicts it.
const V3: &str = "
CREATE TABLE articles_v3 (
    id                     TEXT PRIMARY KEY,
    source_id              TEXT REFERENCES sources(id) ON DELETE SET NULL,
    source_name            TEXT NOT NULL,
    source_type            TEXT NOT NULL CHECK (source_type IN ('rss','direct')),
    title                  TEXT NOT NULL,
    link                   TEXT NOT NULL,
    excerpt                TEXT NOT NULL,
    content_html           TEXT NOT NULL,
    hero_image_path        TEXT,
    published_at           TEXT,
    fetched_at             TEXT NOT NULL,
    read_time_min          INTEGER NOT NULL DEFAULT 1,
    reading_state          TEXT NOT NULL DEFAULT 'unread' CHECK (reading_state IN ('unread','reading','read')),
    favorited              INTEGER NOT NULL DEFAULT 0,
    zim_path               TEXT,
    zim_main_path          TEXT,
    extraction_confident    INTEGER NOT NULL DEFAULT 1,
    reading_progress        REAL NOT NULL DEFAULT 0,
    content_zim_path        TEXT,
    archive_source           TEXT NOT NULL DEFAULT 'server' CHECK (archive_source IN ('server','local_legacy')),
    archive_status           TEXT NOT NULL DEFAULT 'pending' CHECK (archive_status IN ('pending','ready','failed')),
    archive_last_error       TEXT,
    archive_last_opened_at   TEXT,
    updated_at               TEXT NOT NULL
);
INSERT INTO articles_v3 (
    id, source_id, source_name, source_type, title, link, excerpt,
    content_html, hero_image_path, published_at, fetched_at, read_time_min,
    reading_state, favorited, zim_path, zim_main_path, extraction_confident,
    reading_progress, content_zim_path, archive_source, archive_status,
    archive_last_error, archive_last_opened_at, updated_at
)
SELECT
    id, source_id, source_name, source_type, title, link, excerpt,
    content_html, hero_image_path, published_at, fetched_at, read_time_min,
    CASE
        WHEN unread = 1 THEN 'unread'
        WHEN reading_progress = 0 THEN 'read'
        ELSE 'reading'
    END,
    favorited, zim_path, zim_main_path, extraction_confident,
    reading_progress, NULL, 'local_legacy', 'ready', NULL, NULL, updated_at
FROM articles;
DROP TABLE articles;
ALTER TABLE articles_v3 RENAME TO articles;

CREATE INDEX idx_articles_source_id ON articles(source_id);
CREATE INDEX idx_articles_fetched_at ON articles(fetched_at DESC);
CREATE INDEX idx_articles_reading_state ON articles(reading_state);
CREATE UNIQUE INDEX idx_articles_link ON articles(link);

INSERT INTO settings (key, value) VALUES
    ('archive_server_url', ''),
    ('archive_server_token', '')
ON CONFLICT(key) DO NOTHING;
";

// Removes server-side full-page archiving entirely (the app now offers
// only the offline readable view plus a plain external link to the
// original page — no in-app archived-page viewer). Drops the
// full-archive bookkeeping columns (`zim_path`, `zim_main_path`,
// `archive_source`, `archive_status`, `archive_last_error`,
// `archive_last_opened_at`) and the `archive_server_url`/
// `archive_server_token` settings; `content_zim_path` (the small,
// never-evicted zim holding just the readable view's own images) is
// untouched — that's what keeps the readable view working offline.
//
// `local_legacy` rows (pre-server-side-archiving articles, migrated by
// V3) had no `content_zim_path` of their own — their one full archive
// served both roles. Dropping `zim_path` here means those specific
// rows' readable-view images stop resolving offline; accepted the same
// way V2's dup-collapse was (personal, pre-remodel data), rather than
// engineered around with a backfill that would require re-fetching
// every such article's images from the live network during migration.
const V4: &str = "
 CREATE TABLE articles_v4 (
     id                   TEXT PRIMARY KEY,
     source_id            TEXT REFERENCES sources(id) ON DELETE SET NULL,
     source_name          TEXT NOT NULL,
     source_type          TEXT NOT NULL CHECK (source_type IN ('rss','direct')),
     title                TEXT NOT NULL,
     link                 TEXT NOT NULL,
     excerpt              TEXT NOT NULL,
     content_html         TEXT NOT NULL,
     hero_image_path      TEXT,
     published_at         TEXT,
     fetched_at           TEXT NOT NULL,
     read_time_min        INTEGER NOT NULL DEFAULT 1,
     reading_state        TEXT NOT NULL DEFAULT 'unread' CHECK (reading_state IN ('unread','reading','read')),
     favorited            INTEGER NOT NULL DEFAULT 0,
     extraction_confident INTEGER NOT NULL DEFAULT 1,
     reading_progress     REAL NOT NULL DEFAULT 0,
     content_zim_path     TEXT,
     updated_at           TEXT NOT NULL
 );
 INSERT INTO articles_v4 (
     id, source_id, source_name, source_type, title, link, excerpt,
     content_html, hero_image_path, published_at, fetched_at, read_time_min,
     reading_state, favorited, extraction_confident, reading_progress,
     content_zim_path, updated_at
 )
 SELECT
     id, source_id, source_name, source_type, title, link, excerpt,
     content_html, hero_image_path, published_at, fetched_at, read_time_min,
     reading_state, favorited, extraction_confident, reading_progress,
     content_zim_path, updated_at
 FROM articles;
 DROP TABLE articles;
 ALTER TABLE articles_v4 RENAME TO articles;

 CREATE INDEX idx_articles_source_id ON articles(source_id);
 CREATE INDEX idx_articles_fetched_at ON articles(fetched_at DESC);
 CREATE INDEX idx_articles_reading_state ON articles(reading_state);
 CREATE UNIQUE INDEX idx_articles_link ON articles(link);

 DELETE FROM settings WHERE key IN ('archive_server_url', 'archive_server_token');
";

// Adds a `tags` column, populated from RSS `<category>` elements
// (`feed_rs::Entry::categories`) at capture time — real per-article data,
// not a fixed/hardcoded taxonomy. A plain `ADD COLUMN` suffices here (no
// `CHECK` constraint involved, unlike V2-V4's renames), so this is the
// first migration that doesn't need the recreate-and-swap dance.
// Direct-link articles have no feed to draw categories from and simply
// keep the column's default, `'[]'`.
const V5: &str = "
ALTER TABLE articles ADD COLUMN tags TEXT NOT NULL DEFAULT '[]';
";

// Drops `content_zim_path`: content images are no longer packed into a
// per-article ZIM archive (see `capture::archive`), just written as plain
// files under `content/<id>/` — a path convention, not something that
// needs its own column. `content/<id>/` is served by the `legere-content`
// protocol (`content_server.rs`), which validates `<id>` against the
// `articles` table directly rather than reading this column.
//
// Existing rows' `content_html` still has `legere-zim:/<id>/<path>` tokens
// pointing at the old (now-defunct) `zim://` protocol, and their
// `content/<id>.zim` files are now orphans. Left broken rather than
// migrated: this is a personal, pre-remodel database, same call V2's
// dup-collapse and V4's `zim_path` drop already made, rather than
// engineered around with a migration-time re-fetch of every old article's
// images. `gc::sweep_orphaned_files` still removes the stale `.zim` files
// themselves (see its own updated sweep logic) — they just aren't
// re-localized into the new format.
const V6: &str = "
ALTER TABLE articles DROP COLUMN content_zim_path;
";

// Supports the keyset-paginated `list_articles_page` query (replacing the
// old "fetch every article" `list_articles`, which stopped scaling once
// libraries reached hundreds/thousands of rows): the library and
// favorites views query with a `WHERE favorited = ...` predicate on every
// page fetch rather than filtering an already-fully-loaded array in the
// frontend. The real category/folder filter is added with its own index in
// V9 rather than riding along on `idx_articles_fetched_at`.
const V7: &str = "
CREATE INDEX idx_articles_favorited ON articles(favorited);
CREATE INDEX idx_articles_source_name ON articles(source_name);
";

// Normalizes every existing row's `tags` JSON array to the invariant new
// writes have enforced in Rust since `db::queries::normalize_tags` was
// introduced (lowercase, trimmed, de-duplicated): RSS `<category>`
// elements and Raindrop-import tag cells were stored verbatim before
// this, so pre-existing rows can carry mixed-case or duplicate entries
// that a same-tag filter/manual edit wouldn't otherwise recognize as the
// same tag. Done here in SQL (rather than left to `normalize_tags` at the
// next write) because a row that's never rewritten again would otherwise
// carry stale casing indefinitely — `list_tags`' sidebar counts and the
// tag filter both read straight off this column.
//
// The inner `SELECT DISTINCT ... ORDER BY t` collapses case-insensitive
// duplicates (e.g. `Rust` and `rust`) before `json_group_array` rebuilds
// the array; an all-empty or already-`'[]'` source array correctly comes
// back as `'[]'` via `COALESCE` (an empty `json_each` produces zero
// aggregate rows, and `json_group_array` of zero rows is `NULL`, not
// `'[]'`).
const V8: &str = "
 UPDATE articles
 SET tags = (
     SELECT COALESCE(json_group_array(t), '[]')
     FROM (
         SELECT DISTINCT lower(trim(je.value)) AS t
         FROM json_each(articles.tags) AS je
         WHERE trim(je.value) != ''
         ORDER BY t
     )
 );
";

// Introduces real, user-managed categories — flat, like folders, no
// nesting — replacing the sidebar's old stand-in of grouping by
// `source_name` (see `queries::list_categories`' pre-V9 doc comment,
// since rewritten). A category is a first-class row so it can exist with
// zero articles (e.g. after every article in it is reassigned), which a
// `SELECT DISTINCT` over `articles` could never represent.
//
// `articles.category_id` is nullable with `ON DELETE SET NULL`: deleting
// a category un-categorizes its articles rather than deleting them, and
// an article with no category is a real, permitted state ("Uncategorized"
// in the UI) — not a migration artifact to backfill away. Existing rows
// deliberately get `category_id = NULL` rather than a category synthesized
// from their old `source_name` (e.g. no auto-created "Raindrop import"
// category): `source_name` was never a real category, just this table's
// placeholder for one, so carrying it forward as if it were real would
// reintroduce the exact default the UI is being changed to stop assuming.
//
// `name` is unique case-insensitively (`COLLATE NOCASE`) so "Recipes" and
// "recipes" can't both exist as distinct categories a user would have to
// notice and merge by hand.
const V9: &str = "
CREATE TABLE categories (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL COLLATE NOCASE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE UNIQUE INDEX idx_categories_name ON categories(name);

ALTER TABLE articles ADD COLUMN category_id TEXT REFERENCES categories(id) ON DELETE SET NULL;
CREATE INDEX idx_articles_category_id ON articles(category_id);
";

// Per-article reading-appearance overrides, all nullable — NULL means
// "no override, follow the global setting" (`db::queries::get_settings`),
// not "unset to a default value". Added so opening an article and
// changing its typography/theme no longer silently rewrites the global
// default for every other article (the previous behavior, and the bug
// this migration exists to fix) — see `commands::articles::set_reading_overrides`.
//
// `theme_override` also folds in what used to be the separate
// `reader_theme` global setting (`light`/`sepia`/`dark`): the app now has
// exactly one theme axis (`light`/`dark`, `Settings.app_theme`), and an
// article can only override it to the other of those same two values —
// `sepia` is dropped entirely, not migrated forward. The now-orphaned
// `reader_theme` row in `settings` is deleted rather than left inert.
const V10: &str = "
ALTER TABLE articles ADD COLUMN font_size_override INTEGER;
ALTER TABLE articles ADD COLUMN measure_override TEXT;
ALTER TABLE articles ADD COLUMN leading_override TEXT;
ALTER TABLE articles ADD COLUMN theme_override TEXT;

DELETE FROM settings WHERE key = 'reader_theme';
";

// Drops `source_name`: originally captured provenance for display
// ("Raindrop import", "Direct link", or an RSS feed's title at capture
// time) but it never served any purpose beyond that label once real
// categories (V9) took over folder/grouping duties — the reader/library
// UI has stopped rendering it in favor of the article's live category
// name (`ArticleSummary`/`ArticleDetail`'s `category_name`, resolved via
// the `categories` join). The index `idx_articles_source_name` (V7) has
// to be dropped first — SQLite's `ALTER TABLE ... DROP COLUMN` refuses to
// drop a column that's still indexed.
//
// `sources.name` (the *feed's* own display name, used for RSS source
// management, kept in sync by `queries::set_source_name_if_default`) is
// a distinct column on a distinct table and is untouched here.
const V11: &str = "
 DROP INDEX idx_articles_source_name;
 ALTER TABLE articles DROP COLUMN source_name;
";

// Every capture path (`capture::capture_local`) now defaults
// `published_at` to capture time whenever the source page didn't expose
// one, rather than storing `NULL` and leaving "what date do we show?" to
// every reader of the column — see that function's doc comment. This
// backfills rows captured before that change existed: `published_at IS
// NULL` becomes "now" (this migration's run time), same fallback the app
// itself uses when a real publish date was never available, just applied
// once retroactively instead of at each row's own original capture time
// (which is gone — the column being NULL *is* that lost information).
const V12: &str = "
UPDATE articles SET published_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE published_at IS NULL;
";

// Lets a category carry a chosen glyph from the frontend's fixed category
// icon pack (`lib/categoryIcons.ts`) instead of the sidebar/card hashing
// its name into a colored dot (`sourceColor.ts`, now removed) — the dot
// gave every category *a* color but no visual identity a user actually
// picked, and read as a "new item" indicator once paired with the
// (also-removed) unread dot next to a title. `'folder'` is the pack's
// generic entry and what every pre-existing category backfills to, same
// as a freshly created one that doesn't specify an icon.
//
// Stored as a free-text id rather than a CHECK-constrained enum: the icon
// pack is a frontend-owned, purely cosmetic list, and pinning it to a
// backend CHECK would mean a migration every time the pack grows. An id
// that doesn't match any known pack entry (e.g. one from a newer version
// opening an older database, though the pack has never shrunk) just falls
// back to the generic folder glyph client-side rather than failing to
// render.
const V13: &str = "
ALTER TABLE categories ADD COLUMN icon TEXT NOT NULL DEFAULT 'folder';
";

// Compresses `content_html` at rest: this was, before this migration, the
// single largest non-image contributor to a library's on-disk size (HTML
// text compresses well with plain DEFLATE). Split into two migrations
// rather than one recreate-and-swap dance (the shape V2-V4 used for a
// CHECK-constraint change) because *this* change needs actual Rust code
// to run against existing rows — there is no SQL gzip function — and
// `rusqlite_migration`'s hook API only supports running Rust code *after*
// a migration's own SQL, not interleaved with a table recreate. V14 adds
// a new nullable `content_html_gz BLOB` column and backfills every
// existing row's compressed bytes into it via `M::up_with_hook`; V15 then
// drops the old `content_html` TEXT column (a plain `ALTER TABLE ... DROP
// COLUMN` suffices — unlike V2-V4's columns, `content_html` was never
// referenced by a CHECK/UNIQUE/index) and renames `content_html_gz` back
// to `content_html`, so every other migration/query in this codebase that
// names the column doesn't also need to change. `content_html_gz` is left
// nullable at the schema level rather than backfilled through a third
// recreate purely to add `NOT NULL`: every write path
// (`db::queries::insert_captured_article` and friends) always supplies a
// value in Rust, so the DB-level constraint would only ever be defense in
// depth, not the actual guarantee — not worth a second full table
// recreate on a 20+ column table for that alone.
//
// Verified against a real (copied, not live) 1,611-article production
// database before this shipped: every row's compressed bytes decompress
// back to exactly what was there before. Note this migration alone does
// not shrink the `.db` file on disk — SQLite's `DROP COLUMN`/`RENAME
// COLUMN` free the old column's pages onto the internal free list but
// don't reclaim the file's size; only a `VACUUM` does that, which isn't
// run automatically here (it rewrites the whole file and needs roughly
// its size again in free disk space — too heavy to run unprompted during
// an app-startup migration). A user who wants the freed space back on
// disk needs to `VACUUM` by hand.
const V14: &str = "
ALTER TABLE articles ADD COLUMN content_html_gz BLOB;
";

fn backfill_compressed_content_html(tx: &rusqlite::Transaction) -> HookResult {
    let mut stmt = tx.prepare("SELECT id, content_html FROM articles")?;
    let rows: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    drop(stmt);
    for (id, html) in rows {
        let compressed = crate::db::compression::compress_html(&html);
        tx.execute(
            "UPDATE articles SET content_html_gz = ?2 WHERE id = ?1",
            rusqlite::params![id, compressed],
        )?;
    }
    Ok(())
}

const V15: &str = "
ALTER TABLE articles DROP COLUMN content_html;
ALTER TABLE articles RENAME COLUMN content_html_gz TO content_html;
";

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(V1),
        M::up(V2),
        M::up(V3),
        M::up(V4),
        M::up(V5),
        M::up(V6),
        M::up(V7),
        M::up(V8),
        M::up(V9),
        M::up(V10),
        M::up(V11),
        M::up(V12),
        M::up(V13),
        M::up_with_hook(V14, backfill_compressed_content_html),
        M::up(V15),
    ])
}

pub fn migrate(conn: &mut Connection) -> Result<(), rusqlite_migration::Error> {
    // See V2's doc comment: `PRAGMA foreign_keys` must be toggled outside
    // any migration's own SQL (each migration already runs inside a
    // transaction, where the pragma is a documented no-op) and restored
    // afterward, since every pooled connection has it on by default
    // (`db::pool::build_pool`) and other code relies on that for
    // `ON DELETE SET NULL` to actually fire during normal operation.
    conn.pragma_update(None, "foreign_keys", false)
        .expect("PRAGMA foreign_keys=OFF should not fail on an open connection");
    let result = migrations().to_latest(conn);
    conn.pragma_update(None, "foreign_keys", true)
        .expect("PRAGMA foreign_keys=ON should not fail on an open connection");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a V1-schema connection with data that exercises every
    /// V2-migration edge case: two articles sharing a link (must collapse
    /// to the newer one), a `mail`-typed article (must remap to
    /// `direct`), and the old `default_font_size` setting (must backfill
    /// `reader_font_size`).
    fn v1_conn_with_test_data() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        // Applied through `rusqlite_migration` itself (not a raw
        // `execute_batch`), so it records `user_version = 1` — otherwise
        // the later `migrate()` call in each test doesn't know V1 was
        // already applied and tries to re-run it against tables that
        // already exist.
        Migrations::new(vec![M::up(V1)])
            .to_latest(&mut conn)
            .expect("apply V1 schema via rusqlite_migration");

        conn.execute_batch(
            "INSERT INTO sources (id, name, type, feed_url, status, article_count, created_at)
             VALUES ('src-1', 'Test Feed', 'rss', 'https://example.com/feed.xml', 'active', 2, '2026-01-01T00:00:00Z');

             INSERT INTO articles (
                 id, source_id, source_name, source_type, title, link, excerpt,
                 content_html, fetched_at, zim_path
             ) VALUES
                 ('art-old', 'src-1', 'Test Feed', 'rss', 'Old capture', 'https://example.com/dup',
                  'old', '<p>old</p>', '2026-01-01T00:00:00Z', 'archives/art-old.zim'),
                 ('art-new', 'src-1', 'Test Feed', 'rss', 'New capture', 'https://example.com/dup',
                  'new', '<p>new</p>', '2026-01-02T00:00:00Z', 'archives/art-new.zim'),
                 ('art-mail', NULL, 'Direct link', 'mail', 'Mail-typed article', 'https://example.com/mail',
                  'mail', '<p>mail</p>', '2026-01-01T00:00:00Z', 'archives/art-mail.zim');",
        )
        .expect("insert V1 test data");

        conn
    }

    #[test]
    fn v2_collapses_duplicate_links_onto_the_newest_row() {
        let mut conn = v1_conn_with_test_data();
        migrate(&mut conn).expect("migrate to V2");

        let mut stmt = conn
            .prepare("SELECT id FROM articles WHERE link = 'https://example.com/dup'")
            .unwrap();
        let ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(ids, vec!["art-new".to_string()]);
    }

    #[test]
    fn v2_remaps_mail_articles_to_direct() {
        let mut conn = v1_conn_with_test_data();
        migrate(&mut conn).expect("migrate to V2");

        let source_type: String = conn
            .query_row(
                "SELECT source_type FROM articles WHERE id = 'art-mail'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(source_type, "direct");
    }

    #[test]
    fn v2_enforces_unique_link_and_new_check_constraints() {
        let mut conn = v1_conn_with_test_data();
        migrate(&mut conn).expect("migrate to V2");

        let duplicate_insert = conn.execute(
            "INSERT INTO articles (
                 id, source_name, source_type, title, link, excerpt,
                 content_html, fetched_at, updated_at
             ) VALUES ('art-dup2', 'Direct link', 'direct', 'x', 'https://example.com/dup',
                       'x', '<p>x</p>', '2026-01-03T00:00:00Z', '2026-01-03T00:00:00Z')",
            [],
        );
        assert!(
            duplicate_insert.is_err(),
            "UNIQUE(link) should reject this insert"
        );

        let mail_source_insert = conn.execute(
            "INSERT INTO sources (id, name, type, feed_url, status, article_count, created_at, updated_at)
             VALUES ('src-mail', 'Bad', 'mail', NULL, 'active', 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        );
        assert!(
            mail_source_insert.is_err(),
            "sources.type CHECK should reject 'mail'"
        );
    }

    #[test]
    fn v2_backfills_reader_font_size_from_default_font_size() {
        let mut conn = v1_conn_with_test_data();
        migrate(&mut conn).expect("migrate to V2");

        let value: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'reader_font_size'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        // v1_conn_with_test_data never overrides the V1-seeded default
        // ('medium'), which maps to '19'.
        assert_eq!(value, "19");
    }

    /// Builds a V2-schema connection with three rows covering every
    /// `unread`/`reading_progress` combination V3's `reading_state`
    /// mapping has to judge: never opened, opened and finished (progress
    /// at 0, the old model's only "done" signal), and opened but
    /// abandoned partway through.
    fn v2_conn_with_test_data() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        Migrations::new(vec![M::up(V1), M::up(V2)])
            .to_latest(&mut conn)
            .expect("apply V1+V2 schema via rusqlite_migration");

        conn.execute_batch(
            "INSERT INTO articles (
                 id, source_name, source_type, title, link, excerpt,
                 content_html, fetched_at, zim_path, zim_main_path, unread, reading_progress, updated_at
             ) VALUES
                 ('art-unread', 'Direct link', 'direct', 'Unread', 'https://example.com/unread',
                  'e', '<p>x</p>', '2026-01-01T00:00:00Z', 'archives/a.zim', 'index.html', 1, 0, '2026-01-01T00:00:00Z'),
                 ('art-finished', 'Direct link', 'direct', 'Finished', 'https://example.com/finished',
                  'e', '<p>x</p>', '2026-01-01T00:00:00Z', 'archives/b.zim', 'index.html', 0, 0, '2026-01-01T00:00:00Z'),
                 ('art-partial', 'Direct link', 'direct', 'Partial', 'https://example.com/partial',
                  'e', '<p>x</p>', '2026-01-01T00:00:00Z', 'archives/c.zim', 'index.html', 0, 0.4, '2026-01-01T00:00:00Z');",
        )
        .expect("insert V2 test data");

        conn
    }

    #[test]
    fn v3_maps_unread_and_reading_progress_onto_reading_state() {
        let mut conn = v2_conn_with_test_data();
        migrate(&mut conn).expect("migrate to V3");

        let reading_state = |id: &str| -> String {
            conn.query_row(
                "SELECT reading_state FROM articles WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .unwrap()
        };
        assert_eq!(reading_state("art-unread"), "unread");
        assert_eq!(reading_state("art-finished"), "read");
        assert_eq!(reading_state("art-partial"), "reading");
    }

    /// V4 drops every full-archive bookkeeping column V3 introduced
    /// (`zim_path`, `archive_source`, `archive_status`, ...) entirely —
    /// the app no longer stores a server-captured full-page snapshot at
    /// all, only the readable view (and its own small, persistent
    /// `content_zim_path`, which this proves survives V4 specifically —
    /// V6 is what eventually drops that one too, see
    /// `v6_drops_content_zim_path_column`).
    #[test]
    fn v4_drops_full_archive_columns_but_keeps_content_zim_path() {
        let mut conn = v2_conn_with_test_data();
        migrations()
            .to_version(&mut conn, 4)
            .expect("migrate to V4");

        let content_zim_path: Option<String> = conn
            .query_row(
                "SELECT content_zim_path FROM articles WHERE id = 'art-unread'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        // V3 leaves every migrated (`local_legacy`) row's own
        // `content_zim_path` NULL — this only proves the column survives
        // V4, not that it's populated here.
        assert_eq!(content_zim_path, None);

        let zim_path_column_gone = conn.query_row(
            "SELECT zim_path FROM articles WHERE id = 'art-unread'",
            [],
            |row| row.get::<_, Option<String>>(0),
        );
        assert!(
            zim_path_column_gone.is_err(),
            "zim_path column should no longer exist after V4"
        );
    }

    #[test]
    fn v4_removes_archive_server_settings() {
        let mut conn = v2_conn_with_test_data();
        migrate(&mut conn).expect("migrate to latest");

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM settings WHERE key IN ('archive_server_url', 'archive_server_token')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "archive server settings should be removed by V4");
    }

    #[test]
    fn v5_adds_tags_column_defaulting_to_an_empty_array() {
        let mut conn = v2_conn_with_test_data();
        migrate(&mut conn).expect("migrate to latest");

        let tags: String = conn
            .query_row(
                "SELECT tags FROM articles WHERE id = 'art-unread'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tags, "[]");
    }

    #[test]
    fn v8_normalizes_existing_tags_to_lowercase_trimmed_deduped() {
        let mut conn = v2_conn_with_test_data();
        migrations()
            .to_version(&mut conn, 5)
            .expect("migrate to V5");
        conn.execute(
            "UPDATE articles SET tags = ?1 WHERE id = 'art-unread'",
            [r#"["Rust", " rust ", "WebDev", ""]"#],
        )
        .expect("seed pre-normalization tags");

        migrate(&mut conn).expect("migrate to latest");

        let tags: String = conn
            .query_row(
                "SELECT tags FROM articles WHERE id = 'art-unread'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tags, r#"["rust","webdev"]"#);
    }

    #[test]
    fn v9_existing_articles_start_uncategorized() {
        let mut conn = v2_conn_with_test_data();
        migrate(&mut conn).expect("migrate to latest");

        let category_id: Option<String> = conn
            .query_row(
                "SELECT category_id FROM articles WHERE id = 'art-unread'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            category_id, None,
            "pre-existing rows must not be backfilled onto a synthesized category"
        );
    }

    #[test]
    fn v9_category_name_is_unique_case_insensitively() {
        let mut conn = v2_conn_with_test_data();
        migrate(&mut conn).expect("migrate to latest");

        conn.execute(
            "INSERT INTO categories (id, name, created_at, updated_at)
             VALUES ('cat-1', 'Recipes', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        )
        .expect("first insert should succeed");

        let dup = conn.execute(
            "INSERT INTO categories (id, name, created_at, updated_at)
             VALUES ('cat-2', 'recipes', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        );
        assert!(
            dup.is_err(),
            "a case-insensitive duplicate name must be rejected"
        );
    }

    #[test]
    fn v9_deleting_a_category_uncategorizes_its_articles_instead_of_deleting_them() {
        let mut conn = v2_conn_with_test_data();
        migrate(&mut conn).expect("migrate to latest");

        conn.execute(
            "INSERT INTO categories (id, name, created_at, updated_at)
             VALUES ('cat-1', 'Recipes', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        )
        .expect("insert category");
        conn.execute(
            "UPDATE articles SET category_id = 'cat-1' WHERE id = 'art-unread'",
            [],
        )
        .expect("assign category");

        conn.execute("DELETE FROM categories WHERE id = 'cat-1'", [])
            .expect("delete category");

        let (still_exists, category_id): (i64, Option<String>) = conn
            .query_row(
                "SELECT 1, category_id FROM articles WHERE id = 'art-unread'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(still_exists, 1, "the article itself must survive");
        assert_eq!(category_id, None, "its category_id must be cleared");
    }

    #[test]
    fn v6_drops_content_zim_path_column() {
        let mut conn = v2_conn_with_test_data();
        migrate(&mut conn).expect("migrate to latest");

        let column_gone = conn.query_row(
            "SELECT content_zim_path FROM articles WHERE id = 'art-unread'",
            [],
            |row| row.get::<_, Option<String>>(0),
        );
        assert!(
            column_gone.is_err(),
            "content_zim_path column should no longer exist after V6"
        );
    }

    #[test]
    fn v11_drops_source_name_column() {
        let mut conn = v2_conn_with_test_data();
        migrate(&mut conn).expect("migrate to latest");

        let column_gone = conn.query_row("SELECT source_name FROM articles LIMIT 1", [], |row| {
            row.get::<_, Option<String>>(0)
        });
        assert!(
            column_gone.is_err(),
            "source_name column should no longer exist after V11"
        );
    }

    #[test]
    fn v12_backfills_null_published_at_but_leaves_a_real_one_alone() {
        let mut conn = v2_conn_with_test_data();
        migrate(&mut conn).expect("migrate to latest");

        conn.execute(
            "UPDATE articles SET published_at = '2020-05-01T00:00:00Z' WHERE id = 'art-finished'",
            [],
        )
        .expect("seed a real published_at");
        conn.execute(
            "UPDATE articles SET published_at = NULL WHERE id = 'art-unread'",
            [],
        )
        .expect("clear published_at back to NULL, as pre-V12 rows had it");

        // Re-running V12 alone (rather than a fresh migrate from V1) mimics
        // what actually happened historically: rows already existed with
        // NULL published_at before this migration was added.
        conn.execute_batch(V12).expect("re-apply V12's backfill");

        let published_at = |id: &str| -> Option<String> {
            conn.query_row(
                "SELECT published_at FROM articles WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .unwrap()
        };
        assert_eq!(
            published_at("art-finished"),
            Some("2020-05-01T00:00:00Z".to_string()),
            "a real published_at must not be overwritten"
        );
        assert!(
            published_at("art-unread").is_some(),
            "a NULL published_at must be backfilled to a non-NULL value"
        );
    }

    #[test]
    fn v2_preserves_foreign_keys_enforcement_after_migrating() {
        let mut conn = v1_conn_with_test_data();
        migrate(&mut conn).expect("migrate to V2");

        let fk_enabled: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            fk_enabled, 1,
            "foreign_keys must be restored to ON after migrating"
        );
    }

    #[test]
    fn v14_v15_compress_existing_plaintext_content_html_without_losing_it() {
        let mut conn = v2_conn_with_test_data();
        migrate(&mut conn).expect("migrate to latest");

        // The column is `content_html` again post-V15 (renamed back from
        // `content_html_gz`), but its *bytes* are now gzip, not plain
        // text — reading it raw must round-trip through
        // `compression::decompress_html` to recover the original HTML
        // `v2_conn_with_test_data` inserted before V14 existed.
        let stored: Vec<u8> = conn
            .query_row(
                "SELECT content_html FROM articles WHERE id = 'art-unread'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(crate::db::compression::decompress_html(&stored), "<p>x</p>");
    }

    #[test]
    fn v15_drops_the_old_plaintext_content_html_and_keeps_one_column() {
        let mut conn = v2_conn_with_test_data();
        migrate(&mut conn).expect("migrate to latest");

        let column_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('articles') WHERE name = 'content_html'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            column_count, 1,
            "there must be exactly one content_html column after V15, not a leftover content_html_gz"
        );
    }
}

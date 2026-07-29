use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

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

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(V1), M::up(V2)])
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
                 content_html, fetched_at, zim_path, updated_at
             ) VALUES ('art-dup2', 'Direct link', 'direct', 'x', 'https://example.com/dup',
                       'x', '<p>x</p>', '2026-01-03T00:00:00Z', 'archives/x.zim', '2026-01-03T00:00:00Z')",
            [],
        );
        assert!(duplicate_insert.is_err(), "UNIQUE(link) should reject this insert");

        let mail_source_insert = conn.execute(
            "INSERT INTO sources (id, name, type, feed_url, status, article_count, created_at, updated_at)
             VALUES ('src-mail', 'Bad', 'mail', NULL, 'active', 0, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        );
        assert!(mail_source_insert.is_err(), "sources.type CHECK should reject 'mail'");
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

    #[test]
    fn v2_preserves_foreign_keys_enforcement_after_migrating() {
        let mut conn = v1_conn_with_test_data();
        migrate(&mut conn).expect("migrate to V2");

        let fk_enabled: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(fk_enabled, 1, "foreign_keys must be restored to ON after migrating");
    }
}

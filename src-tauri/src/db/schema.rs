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

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(V1)])
}

pub fn migrate(conn: &mut Connection) -> Result<(), rusqlite_migration::Error> {
    migrations().to_latest(conn)
}

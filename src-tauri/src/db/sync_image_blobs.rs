//! Local-only ledger backing `remote_sync::engine`'s cross-article image
//! dedup — see schema `V19`'s doc comment and ARCHITECTURE.md's Sync
//! section for the full design. Never synced: this is purely this
//! device's own bookkeeping for deciding what to upload on its *next*
//! push, same category as `sources.feed_etag`/`feed_last_modified`.

use rusqlite::{Connection, OptionalExtension, params};

/// What to do about one image's content hash, decided by
/// [`record_sighting`] from what this device has (or hasn't) uploaded
/// before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashSighting {
    /// Never seen on this device before — embed the bytes literally in
    /// this article's own archive; nothing uploaded standalone.
    First,
    /// Seen exactly once before, on some earlier (possibly already-
    /// pushed) article — this is the moment to promote it to a shared,
    /// content-addressed blob. The caller must upload it (once) and
    /// reference it by pointer from here on, in this article and every
    /// later one.
    JustPromoted,
    /// Already promoted by an earlier sighting — just reference it by
    /// pointer; nothing new to upload.
    AlreadyPromoted,
}

/// Records one sighting of `content_hash` (an image's sha256 hex digest,
/// from `engine::content_hash`) and reports what the caller should do
/// about it. Call once per (article, file) pair during push's image-
/// upload prep, sequentially, before any concurrent network phase starts
/// — never from inside a concurrently-spawned task, since this takes
/// `&Connection` directly and `rusqlite::Connection` isn't `Sync`.
pub fn record_sighting(conn: &Connection, content_hash: &str) -> rusqlite::Result<HashSighting> {
    let promoted: Option<bool> = conn
        .query_row(
            "SELECT promoted FROM sync_image_blobs WHERE content_hash = ?1",
            params![content_hash],
            |row| row.get::<_, i64>(0).map(|v| v != 0),
        )
        .optional()?;

    match promoted {
        None => {
            conn.execute(
                "INSERT INTO sync_image_blobs (content_hash, promoted) VALUES (?1, 0)",
                params![content_hash],
            )?;
            Ok(HashSighting::First)
        }
        Some(false) => {
            conn.execute(
                "UPDATE sync_image_blobs SET promoted = 1 WHERE content_hash = ?1",
                params![content_hash],
            )?;
            Ok(HashSighting::JustPromoted)
        }
        Some(true) => Ok(HashSighting::AlreadyPromoted),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migrated_conn() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        crate::db::schema::migrate(&mut conn).expect("migrate");
        conn
    }

    #[test]
    fn first_sighting_then_promotion_then_already_promoted_forever_after() {
        let conn = migrated_conn();
        assert_eq!(record_sighting(&conn, "abc").unwrap(), HashSighting::First);
        assert_eq!(
            record_sighting(&conn, "abc").unwrap(),
            HashSighting::JustPromoted
        );
        assert_eq!(
            record_sighting(&conn, "abc").unwrap(),
            HashSighting::AlreadyPromoted
        );
        assert_eq!(
            record_sighting(&conn, "abc").unwrap(),
            HashSighting::AlreadyPromoted
        );
    }

    #[test]
    fn different_hashes_are_tracked_independently() {
        let conn = migrated_conn();
        assert_eq!(record_sighting(&conn, "a").unwrap(), HashSighting::First);
        assert_eq!(record_sighting(&conn, "b").unwrap(), HashSighting::First);
        assert_eq!(
            record_sighting(&conn, "a").unwrap(),
            HashSighting::JustPromoted
        );
        assert_eq!(
            record_sighting(&conn, "b").unwrap(),
            HashSighting::JustPromoted,
            "b's own second sighting must promote it independently of a"
        );
    }
}

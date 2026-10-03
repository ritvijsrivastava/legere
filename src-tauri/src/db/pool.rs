use std::path::Path;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

pub type DbPool = Pool<SqliteConnectionManager>;

pub fn build_pool(db_path: &Path) -> Result<DbPool, r2d2::Error> {
    let manager = SqliteConnectionManager::file(db_path).with_init(|conn| {
        // busy_timeout must be set first: it's what makes this connection's
        // own `journal_mode = WAL` wait instead of erroring immediately if
        // another pool connection is concurrently initializing the same
        // fresh database file (each connection's init batch is otherwise
        // racing the others with no wait behavior of its own yet).
        //
        // `synchronous = NORMAL` is the documented-safe pairing with WAL
        // (unlike with the default rollback journal, where NORMAL can
        // corrupt the database on a crash): a transaction is still durable
        // against an application crash, and only risks losing the most
        // recent commits -- never corruption -- on a full OS crash or power
        // loss, an acceptable trade for the fsync-per-commit cost it avoids
        // during a capture/import/sync burst writing many rows in quick
        // succession.
        conn.execute_batch(
            "PRAGMA busy_timeout = 5000;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )
    });
    Pool::builder().max_size(4).build(manager)
}

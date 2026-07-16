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
        conn.execute_batch(
            "PRAGMA busy_timeout = 5000;
             PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;",
        )
    });
    Pool::builder().max_size(4).build(manager)
}

//! One-shot startup cleanup: files under `archives/`/`media/` with no
//! matching database row are removed. This exists for two reasons —
//! `remove_source`'s `ON DELETE SET NULL` never touches article files
//! (read-later semantics, articles survive their source going away), and
//! before `delete_article` existed, deleting a row some other way (or
//! crashing mid-capture) left orphaned files with nothing to ever clean
//! them up. Runs once per app start, off the async runtime so it never
//! delays startup.

use std::path::Path;

use crate::db::queries;
use crate::state::AppState;

/// Directory names under `data_dir` this sweep considers, and whether a
/// file found there but not in the referenced set is safe to remove.
const SWEPT_DIRS: [&str; 3] = ["archives", "media", "content"];

pub async fn sweep_orphaned_files(state: &AppState) {
    let referenced = {
        let pool = state.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let conn = pool.get().map_err(|e| e.to_string())?;
            queries::list_referenced_files(&conn).map_err(|e| e.to_string())
        })
        .await;
        match result {
            Ok(Ok(set)) => set,
            Ok(Err(err)) => {
                tracing::warn!(%err, "orphan sweep: failed to list referenced files, skipping");
                return;
            }
            Err(err) => {
                tracing::warn!(%err, "orphan sweep: db task panicked, skipping");
                return;
            }
        }
    };

    for dir_name in SWEPT_DIRS {
        sweep_dir(&state.data_dir, dir_name, &referenced).await;
    }
}

async fn sweep_dir(data_dir: &Path, dir_name: &str, referenced: &std::collections::HashSet<String>) {
    let dir = data_dir.join(dir_name);
    let mut entries = match tokio::fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(_) => return, // directory doesn't exist yet — nothing to sweep
    };

    loop {
        let entry = match entries.next_entry().await {
            Ok(Some(entry)) => entry,
            Ok(None) => break,
            Err(_) => break,
        };
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        // Stored paths are always `<dir_name>/<file_name>` — constructed
        // via `format!` at capture time, never `Path::join`, so they're
        // forward-slash-separated regardless of host OS.
        let rel_path = format!("{dir_name}/{file_name}");
        if !referenced.contains(&rel_path) {
            tracing::info!(path = %rel_path, "orphan sweep: removing unreferenced file");
            let _ = tokio::fs::remove_file(&path).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::zim_server::ZimCache;
    use tokio::sync::Mutex;

    async fn build_state(data_dir: &Path) -> AppState {
        let db_path = data_dir.join("legere.db");
        let pool = db::build_pool(&db_path).expect("build pool");
        {
            let mut conn = pool.get().expect("get conn");
            db::schema::migrate(&mut conn).expect("migrate");
        }
        AppState {
            pool,
            http_client: reqwest::Client::new(),
            server_http_client: reqwest::Client::new(),
            data_dir: data_dir.to_path_buf(),
            autosync_handle: Mutex::new(None),
            zim_cache: ZimCache::new(),
            last_foreground_sync: std::sync::Mutex::new(None),
        }
    }

    #[tokio::test]
    async fn removes_files_with_no_matching_article_row() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = build_state(tmp.path()).await;

        std::fs::create_dir_all(tmp.path().join("archives")).unwrap();
        std::fs::create_dir_all(tmp.path().join("media")).unwrap();
        std::fs::write(tmp.path().join("archives/orphan.zim"), b"x").unwrap();
        std::fs::write(tmp.path().join("media/orphan.jpg"), b"x").unwrap();

        sweep_orphaned_files(&state).await;

        assert!(!tmp.path().join("archives/orphan.zim").exists());
        assert!(!tmp.path().join("media/orphan.jpg").exists());
    }

    #[tokio::test]
    async fn keeps_files_referenced_by_a_live_article_row() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = build_state(tmp.path()).await;

        std::fs::create_dir_all(tmp.path().join("archives")).unwrap();
        std::fs::create_dir_all(tmp.path().join("media")).unwrap();
        std::fs::write(tmp.path().join("archives/keep.zim"), b"x").unwrap();
        std::fs::write(tmp.path().join("media/keep.jpg"), b"x").unwrap();

        {
            let conn = state.pool.get().unwrap();
            conn.execute(
                "INSERT INTO articles (
                    id, source_name, source_type, title, link, excerpt,
                    content_html, fetched_at, zim_path, hero_image_path, updated_at
                ) VALUES ('a1', 'Direct link', 'direct', 't', 'https://x/1', 'e',
                          '<p>x</p>', '2026-01-01T00:00:00Z', 'archives/keep.zim',
                          'media/keep.jpg', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        }

        sweep_orphaned_files(&state).await;

        assert!(tmp.path().join("archives/keep.zim").exists());
        assert!(tmp.path().join("media/keep.jpg").exists());
    }

    #[tokio::test]
    async fn does_not_panic_when_directories_do_not_exist_yet() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = build_state(tmp.path()).await;
        // No archives/ or media/ dirs created at all.
        sweep_orphaned_files(&state).await;
    }
}

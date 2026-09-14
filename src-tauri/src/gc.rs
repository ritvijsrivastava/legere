//! One-shot startup cleanup: files under `content/`/`media/` with no
//! matching database row are removed. This exists for two reasons —
//! `remove_source`'s `ON DELETE SET NULL` never touches article files
//! (read-later semantics, articles survive their source going away), and
//! before `delete_article` existed, deleting a row some other way (or
//! crashing mid-capture) left orphaned files with nothing to ever clean
//! them up. Runs once per app start, off the async runtime so it never
//! delays startup.

use std::collections::HashSet;
use std::path::Path;

use crate::db::queries;
use crate::state::AppState;

pub async fn sweep_orphaned_files(state: &AppState) {
    let referenced = {
        let pool = state.pool.clone();
        let result = tokio::task::spawn_blocking(move || {
            let conn = pool.get().map_err(|e| e.to_string())?;
            queries::list_referenced_files(&conn).map_err(|e| e.to_string())
        })
        .await;
        match result {
            Ok(Ok(referenced)) => referenced,
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

    sweep_media(&state.data_dir, &referenced.hero_image_paths).await;
    sweep_content(&state.data_dir, &referenced.live_article_ids).await;
}

/// `media/` holds only flat, per-article files (hero image thumbnails,
/// `media/<id>.jpg`) — anything there not in `hero_image_paths` is
/// removed. Also covers what would otherwise be leftover
/// `content/<id>.zim` files from before content images moved out of
/// per-article ZIM archives (see `db::schema`'s V6 migration) — those
/// never appear in `hero_image_paths` either, so this same pass removes
/// them regardless of which swept directory they're actually found in.
async fn sweep_media(data_dir: &Path, hero_image_paths: &HashSet<String>) {
    let dir = data_dir.join("media");
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
        // Stored paths are always `media/<file_name>` — constructed via
        // `format!` at capture time, never `Path::join`, so they're
        // forward-slash-separated regardless of host OS.
        let rel_path = format!("media/{file_name}");
        if !hero_image_paths.contains(&rel_path) {
            tracing::info!(path = %rel_path, "orphan sweep: removing unreferenced file");
            let _ = tokio::fs::remove_file(&path).await;
        }
    }
}

/// `content/` holds one directory per article (`content/<id>/`, see
/// `capture::archive`) plus, on an install that predates the V6
/// migration, stray flat `content/<id>.zim` files — both kinds of entry
/// not matching a live article id are removed. A directory is removed
/// wholesale (`remove_dir_all`): unlike `media/`'s flat per-article
/// files, individual entries inside a content directory were never
/// tracked one by one, so keep-or-remove is a per-article decision, not
/// a per-file one.
async fn sweep_content(data_dir: &Path, live_article_ids: &HashSet<String>) {
    let dir = data_dir.join("content");
    let mut entries = match tokio::fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(_) => return,
    };

    loop {
        let entry = match entries.next_entry().await {
            Ok(Some(entry)) => entry,
            Ok(None) => break,
            Err(_) => break,
        };
        let path = entry.path();
        let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);

        if is_dir {
            let Some(article_id) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !live_article_ids.contains(article_id) {
                tracing::info!(
                    article_id,
                    "orphan sweep: removing unreferenced content directory"
                );
                let _ = tokio::fs::remove_dir_all(&path).await;
            }
        } else {
            // A leftover flat file (e.g. a pre-V6 `<id>.zim`) — content/
            // holds only per-article directories now, so any flat file
            // here is unconditionally an orphan.
            tracing::info!(path = %path.display(), "orphan sweep: removing stale flat file under content/");
            let _ = tokio::fs::remove_file(&path).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
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
            data_dir: data_dir.to_path_buf(),
            autosync_handle: Mutex::new(None),
            last_foreground_sync: std::sync::Mutex::new(None),
            import_cancel: Mutex::new(None),
            pending_update: Default::default(),
            pending_linux_update: Default::default(),
        }
    }

    #[tokio::test]
    async fn removes_files_with_no_matching_article_row() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = build_state(tmp.path()).await;

        std::fs::create_dir_all(tmp.path().join("content/orphan-id")).unwrap();
        std::fs::create_dir_all(tmp.path().join("media")).unwrap();
        std::fs::write(tmp.path().join("content/orphan-id/photo.jpg"), b"x").unwrap();
        std::fs::write(tmp.path().join("media/orphan.jpg"), b"x").unwrap();

        sweep_orphaned_files(&state).await;

        assert!(!tmp.path().join("content/orphan-id").exists());
        assert!(!tmp.path().join("media/orphan.jpg").exists());
    }

    #[tokio::test]
    async fn removes_stale_flat_zim_files_left_over_from_before_the_v6_migration() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = build_state(tmp.path()).await;

        std::fs::create_dir_all(tmp.path().join("content")).unwrap();
        std::fs::write(tmp.path().join("content/old-article.zim"), b"x").unwrap();

        sweep_orphaned_files(&state).await;

        assert!(!tmp.path().join("content/old-article.zim").exists());
    }

    #[tokio::test]
    async fn keeps_files_referenced_by_a_live_article_row() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = build_state(tmp.path()).await;

        std::fs::create_dir_all(tmp.path().join("content/a1")).unwrap();
        std::fs::create_dir_all(tmp.path().join("media")).unwrap();
        std::fs::write(tmp.path().join("content/a1/photo.jpg"), b"x").unwrap();
        std::fs::write(tmp.path().join("media/keep.jpg"), b"x").unwrap();

        {
            let conn = state.pool.get().unwrap();
            conn.execute(
                "INSERT INTO articles (
                    id, source_type, title, link, excerpt,
                    content_html, fetched_at, hero_image_path, updated_at
                ) VALUES ('a1', 'direct', 't', 'https://x/1', 'e',
                          '<p>x</p>', '2026-01-01T00:00:00Z',
                          'media/keep.jpg', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        }

        sweep_orphaned_files(&state).await;

        assert!(tmp.path().join("content/a1/photo.jpg").exists());
        assert!(tmp.path().join("media/keep.jpg").exists());
    }

    #[tokio::test]
    async fn does_not_panic_when_directories_do_not_exist_yet() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = build_state(tmp.path()).await;
        // No content/ or media/ dirs created at all.
        sweep_orphaned_files(&state).await;
    }
}

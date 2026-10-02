//! Orphaned bucket blob sweep: deletes objects under `legere-sync/blobs/`
//! that no live manifest entry references anymore. Mirrors
//! `crate::gc::sweep_orphaned_files`'s local equivalent, for the same
//! reason — races, retries, and bugs (the "categorys" pluralization typo
//! this module exists to help clean up after is a real example) can
//! leave objects behind that nothing will ever reference again.
//!
//! Runs at the end of every successful sync pass (`engine::try_sync_once`),
//! using that pass's own final, just-written manifest — never a stale one
//! fetched separately, since this device's own pending pushes have to be
//! reflected as "live" or it would delete what it just uploaded.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::client::S3Client;
use super::engine::{SHARED_IMAGES_PREFIX, SyncError, live_blob_keys};
use super::manifest::Manifest;

const BLOBS_PREFIX: &str = "legere-sync/blobs/";

/// How recently an object can have been written and still be left alone
/// even if it looks orphaned. Without this, a blob another device just
/// uploaded — but hasn't yet registered in a manifest write of its own —
/// would look exactly like a real orphan to this device's sweep and get
/// deleted out from under it. Matches the same "leave a grace window
/// before treating something as truly gone" shape as the 60-day
/// tombstone retention (`manifest::TOMBSTONE_RETENTION`), just on a much
/// shorter timescale appropriate to a single sync pass.
const GC_GRACE_PERIOD: Duration = Duration::hours(1);

/// How many keys go into a single `DeleteObjects` request — matches the
/// S3 batch-delete API's own hard per-request cap, so a library with more
/// orphans than this just needs more than one request, never a different
/// code path.
const DELETE_BATCH_SIZE: usize = 1000;

/// Deletes every blob under `legere-sync/blobs/` that isn't referenced by
/// `manifest`'s live entries and hasn't been modified within
/// `GC_GRACE_PERIOD`, batching up to `DELETE_BATCH_SIZE` keys per
/// `DeleteObjects` request (`S3Client::delete_objects`) instead of one
/// `DELETE` per orphan — a real library can accumulate thousands of
/// stale blobs (old captures' superseded images, aborted pushes, ...),
/// and deleting them one request at a time was adding exactly the kind
/// of request-count bloat this whole sync rework exists to avoid.
/// Everything under [`SHARED_IMAGES_PREFIX`] is skipped entirely, never
/// even considered: nothing tracks which articles' archives still
/// reference a given shared blob by pointer (see
/// `engine::ArchiveEntryBody::Pointer`), so there is no safe way to tell
/// a truly-unreferenced one apart from one several other articles still
/// point at — treating all of them as permanently live is the simple,
/// safe default; the set is expected to stay small (one entry per
/// *distinct* deduplicated asset, not per article that reuses it).
/// Returns how many were deleted. A failure partway through (one bad
/// batch) stops the sweep but doesn't fail the sync pass that triggered
/// it — see this module's caller in `engine.rs`.
pub async fn sweep_orphaned_blobs(
    client: &S3Client,
    manifest: &Manifest,
    cancel: &Arc<AtomicBool>,
) -> Result<usize, SyncError> {
    let live_keys: HashSet<String> = live_blob_keys(manifest);

    let objects = client.list_objects_with_prefix(BLOBS_PREFIX).await?;
    let now = Utc::now();
    let orphan_keys: Vec<String> = objects
        .into_iter()
        .filter(|object| {
            !object.key.starts_with(SHARED_IMAGES_PREFIX)
                && !live_keys.contains(&object.key)
                && is_older_than_grace_period(&object.last_modified, now)
        })
        .map(|object| object.key)
        .collect();

    let mut deleted = 0;
    for chunk in orphan_keys.chunks(DELETE_BATCH_SIZE) {
        if cancel.load(Ordering::Relaxed) {
            return Err(SyncError::Cancelled);
        }
        client.delete_objects(chunk).await?;
        deleted += chunk.len();
    }

    Ok(deleted)
}

fn is_older_than_grace_period(last_modified: &str, now: DateTime<Utc>) -> bool {
    match DateTime::parse_from_rfc3339(last_modified) {
        Ok(dt) => now.signed_duration_since(dt.with_timezone(&Utc)) > GC_GRACE_PERIOD,
        // An unparseable timestamp is treated as "too recent to touch" —
        // the safe default when we can't tell, matching
        // `manifest::purge_expired_tombstones`'s same choice.
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_object_is_not_old_enough_to_delete() {
        let now = Utc::now();
        let recent = now.to_rfc3339();
        assert!(!is_older_than_grace_period(&recent, now));
    }

    #[test]
    fn object_older_than_the_grace_period_is_eligible() {
        let now = Utc::now();
        let old = (now - Duration::hours(2)).to_rfc3339();
        assert!(is_older_than_grace_period(&old, now));
    }

    #[test]
    fn unparseable_timestamp_is_treated_as_too_recent_to_touch() {
        let now = Utc::now();
        assert!(!is_older_than_grace_period("not-a-timestamp", now));
    }

    mod sweep {
        //! End-to-end coverage of `sweep_orphaned_blobs` itself (not just
        //! its pure helpers above) against an in-process mock S3 server:
        //! proves an old, unreferenced blob is deleted, a live one
        //! (including an article's image-archive key — the thing
        //! `live_blob_keys` was added to fix, see `engine.rs`'s own
        //! regression test) survives, and a too-recent orphan survives
        //! its grace period. Also the one place the batched
        //! `S3Client::delete_objects` path (`DeleteObjects`/`POST ?delete=1`)
        //! gets exercised against a real HTTP round trip, since
        //! `engine.rs`'s own mock server never seeds old-enough blobs to
        //! trigger an actual delete.

        use std::collections::HashMap;
        use std::sync::Mutex;

        use axum::Router;
        use axum::body::Bytes;
        use axum::extract::{Path as AxumPath, Query, State};
        use axum::http::StatusCode;
        use axum::response::IntoResponse;
        use axum::routing::get;

        use super::*;
        use crate::remote_sync::client::{BucketConfig, S3Client};
        use crate::remote_sync::engine::article_images_key;
        use crate::remote_sync::manifest::{EntityType, ManifestEntry};

        type Store = Arc<Mutex<HashMap<String, String>>>;

        async fn spawn_mock_s3(seed: HashMap<String, String>) -> (String, Store) {
            let store: Store = Arc::new(Mutex::new(seed));

            async fn list_objects(
                State(store): State<Store>,
                AxumPath(_bucket): AxumPath<String>,
                Query(params): Query<HashMap<String, String>>,
            ) -> impl IntoResponse {
                let prefix = params.get("prefix").cloned().unwrap_or_default();
                let guard = store.lock().unwrap();
                let contents: String = guard
                    .iter()
                    .filter(|(k, _)| k.starts_with(&prefix))
                    .map(|(k, last_modified)| {
                        format!(
                            "<Contents><Key>{k}</Key><ETag>&quot;x&quot;</ETag><LastModified>{last_modified}</LastModified><Size>1</Size></Contents>"
                        )
                    })
                    .collect();
                let body = format!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?><ListBucketResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">{contents}</ListBucketResult>"
                );
                (StatusCode::OK, [("content-type", "application/xml")], body)
            }

            async fn delete_objects(
                State(store): State<Store>,
                AxumPath(_bucket): AxumPath<String>,
                body: Bytes,
            ) -> impl IntoResponse {
                // A hand-rolled extraction of every `<Key>...</Key>` in the
                // `DeleteObjects` XML request body — good enough for this
                // mock, no need to pull in a full XML parser just to read
                // back what `rusty_s3::actions::DeleteObjects::body_with_md5`
                // itself just generated.
                let text = String::from_utf8_lossy(&body);
                let mut guard = store.lock().unwrap();
                for segment in text.split("<Key>").skip(1) {
                    if let Some(end) = segment.find("</Key>") {
                        guard.remove(&segment[..end]);
                    }
                }
                (
                    StatusCode::OK,
                    [("content-type", "application/xml")],
                    "<DeleteResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\"></DeleteResult>",
                )
            }

            let app = Router::new()
                .route("/{bucket}/", get(list_objects).post(delete_objects))
                .with_state(store.clone());

            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            tokio::spawn(async move {
                axum::serve(listener, app).await.unwrap();
            });
            (format!("http://localhost:{}", addr.port()), store)
        }

        fn test_client(base_url: &str) -> S3Client {
            S3Client::new(&BucketConfig {
                endpoint: base_url.parse().unwrap(),
                bucket_name: "test-bucket".to_string(),
                region: "auto".to_string(),
                use_path_style: true,
                access_key: "key".to_string(),
                secret_key: "secret".to_string(),
            })
            .unwrap()
        }

        fn article_entry(id: &str) -> ManifestEntry {
            ManifestEntry {
                id: id.to_string(),
                entity_type: EntityType::Article,
                conflict_key: format!("https://example.com/{id}"),
                updated_at: "2026-01-01T00:00:00Z".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                content_hash: "deadbeef".to_string(),
            }
        }

        #[tokio::test]
        async fn deletes_old_orphans_but_keeps_live_keys_and_too_recent_ones() {
            let now = Utc::now();
            let old = (now - Duration::hours(2)).to_rfc3339();
            let recent = now.to_rfc3339();

            let mut seed = HashMap::new();
            // Live article: both its meta blob and image archive must
            // survive, regardless of age.
            seed.insert(
                "legere-sync/blobs/articles/art-live/meta.json.gz".to_string(),
                old.clone(),
            );
            seed.insert(article_images_key("art-live"), old.clone());
            // A real orphan: old enough, not referenced by any live entry.
            seed.insert(
                "legere-sync/blobs/articles/art-gone/meta.json.gz".to_string(),
                old.clone(),
            );
            // Looks orphaned too, but too recent — another device may
            // have just uploaded it and not yet written its own manifest.
            seed.insert(
                "legere-sync/blobs/articles/art-brand-new/meta.json.gz".to_string(),
                recent,
            );

            let (base_url, store) = spawn_mock_s3(seed).await;
            let client = test_client(&base_url);
            let manifest = Manifest {
                version: 1,
                entries: vec![article_entry("art-live")],
                tombstones: vec![],
            };
            let cancel = Arc::new(AtomicBool::new(false));

            let deleted = sweep_orphaned_blobs(&client, &manifest, &cancel)
                .await
                .unwrap();

            assert_eq!(deleted, 1, "only the one true orphan must be deleted");
            let guard = store.lock().unwrap();
            assert!(
                guard.contains_key("legere-sync/blobs/articles/art-live/meta.json.gz"),
                "a live article's metadata must survive"
            );
            assert!(
                guard.contains_key(&article_images_key("art-live")),
                "a live article's image archive must survive — the bug this module's \
                 `live_blob_keys` fixes"
            );
            assert!(
                !guard.contains_key("legere-sync/blobs/articles/art-gone/meta.json.gz"),
                "a real, aged orphan must be deleted"
            );
            assert!(
                guard.contains_key("legere-sync/blobs/articles/art-brand-new/meta.json.gz"),
                "an orphan-looking blob inside its grace period must survive"
            );
        }
    }
}

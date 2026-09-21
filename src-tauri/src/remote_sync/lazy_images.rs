//! On-demand, single-article image fetch — separate from
//! `engine::pull_articles` (which only ever pulls metadata). Eagerly
//! pulling every synced article's images up front would defeat the
//! point of syncing metadata quickly for a large library (see
//! ARCHITECTURE.md's Sync section on the eager-metadata/lazy-images
//! split), so a pulled article's images are only fetched the first time
//! it's actually opened for reading — see
//! `commands::articles::open_for_reading`.

use std::path::Path;

use super::client::S3Client;

/// A no-op if `content/<id>/` already exists locally — the heuristic for
/// "this device already has this article's images," however they got
/// there (a real local capture, or an earlier lazy pull). Best-effort
/// throughout: a missing hero blob or a failed individual image write is
/// skipped, not an error, since this must never block opening an article
/// just because the network hiccupped — matches the offline-first
/// principle everywhere else in the app.
pub async fn pull_article_images(
    client: &S3Client,
    data_dir: &Path,
    id: &str,
    hero_image_path: Option<&str>,
) {
    let content_dir = data_dir.join("content").join(id);
    if tokio::fs::try_exists(&content_dir).await.unwrap_or(false) {
        return;
    }

    if let Some(hero) = hero_image_path {
        let hero_path = data_dir.join(hero);
        let already_have_hero = tokio::fs::try_exists(&hero_path).await.unwrap_or(false);
        if !already_have_hero {
            let key = format!("legere-sync/blobs/articles/{id}/hero.jpg");
            if let Ok(Some((bytes, _))) = client.get_object(&key).await {
                if let Some(parent) = hero_path.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                let _ = tokio::fs::write(&hero_path, bytes).await;
            }
        }
    }

    let prefix = format!("legere-sync/blobs/articles/{id}/images/");
    let Ok(objects) = client.list_objects_with_prefix(&prefix).await else {
        return;
    };
    for object in objects {
        let Some(relative) = object.key.strip_prefix(&prefix) else {
            continue;
        };
        let Ok(Some((bytes, _))) = client.get_object(&object.key).await else {
            continue;
        };
        let dest = content_dir.join(relative);
        if let Some(parent) = dest.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let _ = tokio::fs::write(&dest, bytes).await;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use axum::Router;
    use axum::extract::{Path as AxumPath, Query, State};
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::routing::get;

    use super::*;
    use crate::remote_sync::client::{BucketConfig, S3Client};

    type Store = Arc<Mutex<HashMap<String, Vec<u8>>>>;

    /// A read-only slice of the mock S3 server used elsewhere in this
    /// crate (see `engine.rs`'s own `spawn_mock_s3`), kept separate and
    /// minimal here (GET only, no conditional-write handling) since
    /// `lazy_images` never writes to the bucket, only reads from it.
    async fn spawn_mock_s3(seed: HashMap<String, Vec<u8>>) -> String {
        let store: Store = Arc::new(Mutex::new(seed));

        async fn get_object(
            State(store): State<Store>,
            AxumPath((_bucket, key)): AxumPath<(String, String)>,
        ) -> impl axum::response::IntoResponse {
            match store.lock().unwrap().get(&key) {
                Some(bytes) => (StatusCode::OK, [("etag", "x")], bytes.clone()).into_response(),
                None => StatusCode::NOT_FOUND.into_response(),
            }
        }

        // A minimal hand-rolled `ListObjectsV2` response — just enough XML
        // for `rusty_s3::actions::ListObjectsV2::parse_response` to accept,
        // filtered by the `prefix` query param the same way a real bucket
        // would be. No pagination (`pull_article_images` never lists
        // enough objects per article to need it).
        async fn list_objects(
            State(store): State<Store>,
            AxumPath(_bucket): AxumPath<String>,
            Query(params): Query<HashMap<String, String>>,
        ) -> impl axum::response::IntoResponse {
            let prefix = params.get("prefix").cloned().unwrap_or_default();
            let guard = store.lock().unwrap();
            let contents: String = guard
                .keys()
                .filter(|k| k.starts_with(&prefix))
                .map(|k| {
                    format!(
                        "<Contents><Key>{k}</Key><ETag>&quot;x&quot;</ETag><LastModified>2024-01-01T00:00:00.000Z</LastModified><Size>1</Size></Contents>"
                    )
                })
                .collect();
            let body = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><ListBucketResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">{contents}</ListBucketResult>"
            );
            (StatusCode::OK, [("content-type", "application/xml")], body).into_response()
        }

        let app = Router::new()
            .route("/{bucket}/{*key}", get(get_object))
            .route("/{bucket}/", get(list_objects))
            .with_state(store);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://localhost:{}", addr.port())
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

    #[tokio::test]
    async fn downloads_hero_and_every_content_image_when_missing_locally() {
        let mut seed = HashMap::new();
        seed.insert(
            "legere-sync/blobs/articles/art-1/hero.jpg".to_string(),
            b"hero bytes".to_vec(),
        );
        seed.insert(
            "legere-sync/blobs/articles/art-1/images/sub/pic.jpg".to_string(),
            b"pic bytes".to_vec(),
        );
        let base_url = spawn_mock_s3(seed).await;
        let client = test_client(&base_url);
        let data_dir = tempfile::tempdir().unwrap();

        pull_article_images(&client, data_dir.path(), "art-1", Some("media/art-1.jpg")).await;

        assert_eq!(
            tokio::fs::read(data_dir.path().join("media/art-1.jpg")).await.unwrap(),
            b"hero bytes"
        );
        assert_eq!(
            tokio::fs::read(data_dir.path().join("content/art-1/sub/pic.jpg")).await.unwrap(),
            b"pic bytes"
        );
    }

    #[tokio::test]
    async fn does_nothing_if_the_content_directory_already_exists() {
        let mut seed = HashMap::new();
        seed.insert(
            "legere-sync/blobs/articles/art-1/hero.jpg".to_string(),
            b"hero bytes".to_vec(),
        );
        let base_url = spawn_mock_s3(seed).await;
        let client = test_client(&base_url);
        let data_dir = tempfile::tempdir().unwrap();
        tokio::fs::create_dir_all(data_dir.path().join("content/art-1")).await.unwrap();

        pull_article_images(&client, data_dir.path(), "art-1", Some("media/art-1.jpg")).await;

        assert!(
            !tokio::fs::try_exists(data_dir.path().join("media/art-1.jpg")).await.unwrap(),
            "an already-present content dir must short-circuit before fetching anything"
        );
    }
}

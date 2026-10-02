//! On-demand, single-article image fetch — separate from
//! `engine::pull_articles` (which only ever pulls metadata). Eagerly
//! pulling every synced article's images up front would defeat the
//! point of syncing metadata quickly for a large library (see
//! ARCHITECTURE.md's Sync section on the eager-metadata/lazy-images
//! split), so a pulled article's images are only fetched the first time
//! it's actually opened for reading — see
//! `commands::articles::open_for_reading`.

use std::path::{Path, PathBuf};

use super::client::S3Client;
use super::engine::article_images_key;

/// A no-op if `content/<id>/` already exists locally — the heuristic for
/// "this device already has this article's images," however they got
/// there (a real local capture, or an earlier lazy pull). Best-effort
/// throughout: a missing archive blob, or a failure partway through
/// unpacking it, is skipped, not an error, since this must never block
/// opening an article just because the network hiccupped — matches the
/// offline-first principle everywhere else in the app.
///
/// Fetches the one bundled `content.tar.gz` object `upload_article_images`
/// wrote (see that doc comment, and [`extract_article_archive`]) rather
/// than listing and fetching each image file individually — a single GET
/// instead of a list-then-fetch-N-times round trip.
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

    let Ok(Some((bytes, _))) = client.get_object(&article_images_key(id)).await else {
        return;
    };

    let data_dir = data_dir.to_path_buf();
    let id = id.to_string();
    let hero_image_path = hero_image_path.map(|s| s.to_string());
    let _ = tokio::task::spawn_blocking(move || {
        extract_article_archive(&bytes, &data_dir, &id, hero_image_path.as_deref())
    })
    .await;
}

/// Unpacks `bytes` (a `build_article_archive`-shaped gzip tar) onto disk:
/// its `"hero.jpg"` entry (if any, and only if this article actually has
/// a `hero_image_path`) to `data_dir.join(hero_image_path)`, and every
/// `"content/..."` entry to `data_dir/content/<id>/...`. Synchronous
/// (`std::fs`/`flate2`/`tar` are all blocking) — always run via
/// `tokio::task::spawn_blocking`, never called directly from async code.
/// Best-effort per the caller's doc comment: an unreadable archive or a
/// single bad entry stops extraction early rather than panicking, since
/// a partially-unpacked article is no worse than the pre-sync state (no
/// images at all) and `open_for_reading` tolerates both equally.
fn extract_article_archive(
    bytes: &[u8],
    data_dir: &Path,
    id: &str,
    hero_image_path: Option<&str>,
) -> std::io::Result<()> {
    let decoder = flate2::read::GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(decoder);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let Some(path_str) = path.to_str() else {
            continue;
        };
        let dest: PathBuf = if path_str == "hero.jpg" {
            let Some(hero) = hero_image_path else {
                continue;
            };
            data_dir.join(hero)
        } else if let Some(relative) = path_str.strip_prefix("content/") {
            data_dir.join("content").join(id).join(relative)
        } else {
            continue;
        };
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::File::create(&dest)?;
        std::io::copy(&mut entry, &mut file)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use axum::Router;
    use axum::extract::{Path as AxumPath, State};
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use axum::routing::get;

    use super::*;
    use crate::remote_sync::client::{BucketConfig, S3Client};
    use crate::remote_sync::engine::build_article_archive;

    type Store = Arc<Mutex<HashMap<String, Vec<u8>>>>;

    /// A read-only slice of the mock S3 server used elsewhere in this
    /// crate (see `engine.rs`'s own `spawn_mock_s3`), kept separate and
    /// minimal here (GET only, no conditional-write/listing handling)
    /// since `lazy_images` never writes to the bucket, and only ever GETs
    /// one known key per article (the bundled archive) rather than
    /// listing.
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

        let app = Router::new()
            .route("/{bucket}/{*key}", get(get_object))
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
        let archive = build_article_archive(
            Some(b"hero bytes".to_vec()),
            vec![("sub/pic.jpg".to_string(), b"pic bytes".to_vec())],
        );
        let mut seed = HashMap::new();
        seed.insert(
            "legere-sync/blobs/articles/art-1/content.tar.gz".to_string(),
            archive,
        );
        let base_url = spawn_mock_s3(seed).await;
        let client = test_client(&base_url);
        let data_dir = tempfile::tempdir().unwrap();

        pull_article_images(&client, data_dir.path(), "art-1", Some("media/art-1.jpg")).await;

        assert_eq!(
            tokio::fs::read(data_dir.path().join("media/art-1.jpg"))
                .await
                .unwrap(),
            b"hero bytes"
        );
        assert_eq!(
            tokio::fs::read(data_dir.path().join("content/art-1/sub/pic.jpg"))
                .await
                .unwrap(),
            b"pic bytes"
        );
    }

    #[tokio::test]
    async fn does_nothing_if_the_content_directory_already_exists() {
        let archive = build_article_archive(Some(b"hero bytes".to_vec()), vec![]);
        let mut seed = HashMap::new();
        seed.insert(
            "legere-sync/blobs/articles/art-1/content.tar.gz".to_string(),
            archive,
        );
        let base_url = spawn_mock_s3(seed).await;
        let client = test_client(&base_url);
        let data_dir = tempfile::tempdir().unwrap();
        tokio::fs::create_dir_all(data_dir.path().join("content/art-1"))
            .await
            .unwrap();

        pull_article_images(&client, data_dir.path(), "art-1", Some("media/art-1.jpg")).await;

        assert!(
            !tokio::fs::try_exists(data_dir.path().join("media/art-1.jpg"))
                .await
                .unwrap(),
            "an already-present content dir must short-circuit before fetching anything"
        );
    }
}

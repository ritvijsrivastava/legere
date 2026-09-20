//! A minimal S3-compatible object store client, built on `rusty-s3`
//! (pure-Rust request signing, Sans-IO — it only builds signed URLs, this
//! module does the actual HTTP) and the same bare `reqwest::Client` used
//! by `commands::update`/`update_android`. Unlike `capture`'s fetch path,
//! this does *not* go through `capture::ssrf::ssrf_guarded_client_builder`
//! — the bucket endpoint is infrastructure the user explicitly configured
//! (their own R2/S3/B2/Minio account), not attacker-influenced page
//! content, so SSRF guarding it would only protect against a threat model
//! that doesn't apply here.
//!
//! Every write goes through conditional PUT (`If-Match`/`If-None-Match`)
//! so two devices racing to update `manifest.json` can't silently
//! clobber each other — see [`S3Client::put_object_if_match`]. Providers
//! that don't support conditional writes are rejected at setup time
//! (checked once via [`S3Client::probe_conditional_write_support`]),
//! rather than silently falling back to a weaker, racy mode.

use rusty_s3::{Bucket, BucketError, Credentials, S3Action, UrlStyle, actions};
use std::time::Duration;

/// How long a presigned URL stays valid. Signing happens immediately
/// before each request is sent, so this only needs to outlive one HTTP
/// round trip — kept generous purely to tolerate a slow connection.
const PRESIGN_TTL: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub enum S3Error {
    InvalidBucketConfig(BucketError),
    Request(reqwest::Error),
    /// The provider returned a non-2xx/404/412 status this client doesn't
    /// know how to interpret — carries the status and a snippet of the
    /// body for diagnostics.
    UnexpectedStatus { status: u16, body: String },
}

impl std::fmt::Display for S3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBucketConfig(e) => write!(f, "invalid bucket configuration: {e:?}"),
            Self::Request(e) => write!(f, "request to object store failed: {e}"),
            Self::UnexpectedStatus { status, body } => {
                write!(f, "object store returned unexpected status {status}: {body}")
            }
        }
    }
}

impl std::error::Error for S3Error {}

impl From<reqwest::Error> for S3Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Request(e)
    }
}

/// Everything needed to address a user's bucket. `endpoint` accepts any
/// S3-compatible provider's URL (Cloudflare R2, AWS S3, Backblaze B2,
/// Minio, ...); `use_path_style` should be `true` for most self-hosted/
/// non-AWS providers (Minio, some B2 setups) and `false` for R2/AWS,
/// matching each provider's documented URL convention.
#[derive(Debug, Clone)]
pub struct BucketConfig {
    pub endpoint: url::Url,
    pub bucket_name: String,
    pub region: String,
    pub use_path_style: bool,
    pub access_key: String,
    pub secret_key: String,
}

/// Cheap to clone (an `Arc`-backed `reqwest::Client` plus two small
/// owned structs). Concurrent uploads/downloads clone this once per
/// spawned task rather than sharing a reference, since `tokio::spawn`
/// requires `'static` (see `remote_sync::engine`'s concurrent push/pull
/// phases).
#[derive(Clone)]
pub struct S3Client {
    bucket: Bucket,
    credentials: Credentials,
    http: reqwest::Client,
}

/// One entry from `list_objects_with_prefix`. `last_modified` is the
/// provider's raw RFC 3339 string (not parsed here), since the one
/// caller that needs it (`remote_sync::bucket_gc`) already has its own
/// parse-with-safe-fallback logic and there's no other consumer to share
/// a parsed value with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectInfo {
    pub key: String,
    pub last_modified: String,
}

/// The outcome of a conditional write.
#[derive(Debug, PartialEq, Eq)]
pub enum PutOutcome {
    /// Written successfully; carries the new object's ETag, to be used
    /// as the `If-Match` precondition on the *next* write.
    Written { etag: String },
    /// The precondition failed — someone else wrote (or created, for an
    /// `If-None-Match: *` create-only write) the object first. The
    /// caller must re-fetch and retry its diff/merge against the new
    /// state, per the sync algorithm in ARCHITECTURE.md's Sync section.
    Conflict,
}

impl S3Client {
    pub fn new(config: &BucketConfig) -> Result<Self, S3Error> {
        let style = if config.use_path_style {
            UrlStyle::Path
        } else {
            UrlStyle::VirtualHost
        };
        let bucket = Bucket::new(
            config.endpoint.clone(),
            style,
            config.bucket_name.clone(),
            config.region.clone(),
        )
        .map_err(S3Error::InvalidBucketConfig)?;
        let credentials = Credentials::new(&config.access_key, &config.secret_key);
        Ok(Self {
            bucket,
            credentials,
            http: reqwest::Client::new(),
        })
    }

    /// Fetches an object's bytes and current ETag, or `None` if it
    /// doesn't exist (a fresh bucket with no `manifest.json` yet is the
    /// expected first-sync case, not an error).
    pub async fn get_object(&self, key: &str) -> Result<Option<(Vec<u8>, String)>, S3Error> {
        let action = actions::GetObject::new(&self.bucket, Some(&self.credentials), key);
        let url = action.sign(PRESIGN_TTL);

        let response = self.http.get(url).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(status_error(response).await);
        }

        let etag = etag_header(&response);
        let bytes = response.bytes().await?.to_vec();
        Ok(Some((bytes, etag.unwrap_or_default())))
    }

    /// Writes `body` to `key`, enforced against `precondition`:
    ///
    /// - `Some(etag)` — succeed only if the object's current ETag still
    ///   matches (`If-Match`); use for updating an object this client
    ///   already read.
    /// - `None` — succeed only if the object does not exist yet
    ///   (`If-None-Match: *`); use for the very first write of a new
    ///   key, so two devices can't both "create" it and one silently
    ///   overwrite the other.
    pub async fn put_object_if_match(
        &self,
        key: &str,
        body: Vec<u8>,
        precondition: Option<&str>,
    ) -> Result<PutOutcome, S3Error> {
        let mut action = actions::PutObject::new(&self.bucket, Some(&self.credentials), key);
        match precondition {
            Some(etag) => action.headers_mut().insert("if-match", etag),
            None => action.headers_mut().insert("if-none-match", "*"),
        }
        let url = action.sign(PRESIGN_TTL);

        let mut request = self.http.put(url).body(body);
        request = match precondition {
            Some(etag) => request.header("if-match", etag),
            None => request.header("if-none-match", "*"),
        };

        let response = request.send().await?;
        match response.status() {
            reqwest::StatusCode::PRECONDITION_FAILED => Ok(PutOutcome::Conflict),
            status if status.is_success() => {
                let etag = etag_header(&response).unwrap_or_default();
                Ok(PutOutcome::Written { etag })
            }
            _ => Err(status_error(response).await),
        }
    }

    /// An unconditional write — for per-entity blob objects
    /// (`blobs/articles/<id>/meta.json.gz`, image files, ...), which are
    /// content-addressed/immutable in practice (a given id's content
    /// never changes after capture except via a brand-new capture with a
    /// new id — see ARCHITECTURE.md's capture pipeline), so there is
    /// nothing to race against. Only `manifest.json` needs
    /// [`Self::put_object_if_match`]'s concurrency guard.
    pub async fn put_object(&self, key: &str, body: Vec<u8>) -> Result<(), S3Error> {
        let action = actions::PutObject::new(&self.bucket, Some(&self.credentials), key);
        let url = action.sign(PRESIGN_TTL);
        let response = self.http.put(url).body(body).send().await?;
        if !response.status().is_success() {
            return Err(status_error(response).await);
        }
        Ok(())
    }

    pub async fn delete_object(&self, key: &str) -> Result<(), S3Error> {
        let action = actions::DeleteObject::new(&self.bucket, Some(&self.credentials), key);
        let url = action.sign(PRESIGN_TTL);
        let response = self.http.delete(url).send().await?;
        if !response.status().is_success() && response.status() != reqwest::StatusCode::NOT_FOUND
        {
            return Err(status_error(response).await);
        }
        Ok(())
    }

    /// Lists every object key under `prefix`, paginating through as many
    /// `ListObjectsV2` pages as the bucket returns. Backs
    /// `remote_sync::gc`'s orphaned-blob sweep, the only caller that
    /// needs to enumerate the bucket rather than address a known key
    /// directly.
    pub async fn list_objects_with_prefix(&self, prefix: &str) -> Result<Vec<ObjectInfo>, S3Error> {
        let mut objects = Vec::new();
        let mut continuation_token: Option<String> = None;
        loop {
            let mut action = actions::ListObjectsV2::new(&self.bucket, Some(&self.credentials));
            action.query_mut().insert("prefix", prefix);
            if let Some(token) = &continuation_token {
                action.query_mut().insert("continuation-token", token.as_str());
            }
            let url = action.sign(PRESIGN_TTL);

            let response = self.http.get(url).send().await?;
            if !response.status().is_success() {
                return Err(status_error(response).await);
            }
            let text = response.text().await?;
            let parsed = actions::ListObjectsV2::parse_response(&text)
                .map_err(|e| S3Error::UnexpectedStatus { status: 200, body: e.to_string() })?;

            objects.extend(parsed.contents.into_iter().map(|c| ObjectInfo {
                key: c.key,
                last_modified: c.last_modified,
            }));
            match parsed.next_continuation_token {
                Some(token) => continuation_token = Some(token),
                None => break,
            }
        }
        Ok(objects)
    }

    /// Verifies the provider honors `If-None-Match: *` on `PutObject`
    /// before sync is ever enabled against it — see this module's doc
    /// comment on why an unsupported provider is rejected outright rather
    /// than degraded to a racy fallback. Writes and immediately deletes a
    /// small probe object; safe to call against an otherwise-live bucket.
    pub async fn probe_conditional_write_support(&self) -> Result<bool, S3Error> {
        const PROBE_KEY: &str = "legere-sync/.conditional-write-probe";

        // Clean up any leftover probe object from a previous failed
        // attempt so this check doesn't spuriously report "unsupported"
        // just because the probe itself already exists.
        let _ = self.delete_object(PROBE_KEY).await;

        let first = self
            .put_object_if_match(PROBE_KEY, b"probe".to_vec(), None)
            .await?;
        let second = self
            .put_object_if_match(PROBE_KEY, b"probe-again".to_vec(), None)
            .await?;
        let _ = self.delete_object(PROBE_KEY).await;

        Ok(matches!(first, PutOutcome::Written { .. }) && second == PutOutcome::Conflict)
    }
}

fn etag_header(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim_matches('"').to_string())
}

async fn status_error(response: reqwest::Response) -> S3Error {
    let status = response.status().as_u16();
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "<unreadable body>".to_string());
    S3Error::UnexpectedStatus { status, body }
}

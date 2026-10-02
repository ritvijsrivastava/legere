//! The sync algorithm itself: fetch the manifest, apply tombstones,
//! pull changed rows, resolve `conflict_key` collisions, push local
//! changes, and write the manifest back under a conditional-write
//! concurrency guard, retrying the whole pass if another device raced it
//! — see ARCHITECTURE.md's Sync section for the full design this
//! implements.
//!
//! Article images sync too: pushing an article for the *first* time
//! (see `push_articles`'s `is_new` check) eagerly uploads its hero
//! thumbnail and every file under `content/<id>/` alongside its
//! metadata, since a re-capture (the one thing that ever changes an
//! article's images) always produces a brand-new id — an existing id's
//! images are immutable, so there's nothing to re-upload on a later
//! metadata-only push (a tag edit, a favorite toggle, ...). Pulling
//! devices don't download images as part of a regular sync pass, though
//! — see `remote_sync::lazy_images` for why that's fetched lazily, on
//! first open, instead.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use sha2::{Digest, Sha256};

use crate::db::sync_config::RemoteSyncConfig;
use crate::db::sync_rows::{self, LocalTombstone, SyncArticleRow, SyncCategoryRow, SyncSourceRow};

use super::client::{BucketConfig, PutOutcome, S3Client, S3Error};
use super::conflict::{self, ArticleMutableState, Candidate};
use super::manifest::{EntityType, Manifest, ManifestEntry, TOMBSTONE_RETENTION, TombstoneEntry};

const MANIFEST_KEY: &str = "legere-sync/manifest.json.gz";
const MAX_CONFLICT_RETRIES: u32 = 5;
/// How many blob uploads/downloads run concurrently per phase (push
/// articles, pull categories, ...). Higher than
/// `Settings::import_concurrency`'s 5-10 cap on capture concurrency
/// deliberately: that limit exists to be polite to *other people's*
/// sites being captured from, while sync only ever talks to the user's
/// own bucket, so there's no politeness budget to spend carefully. Not
/// user-configurable (yet) to keep the setup surface small.
const SYNC_CONCURRENCY: usize = 8;

#[derive(Debug)]
pub enum SyncError {
    S3(S3Error),
    Manifest(super::manifest::ManifestError),
    Db(rusqlite::Error),
    /// The bucket keeps rejecting this device's conditional writes with a
    /// conflict `MAX_CONFLICT_RETRIES` times in a row — almost certainly
    /// another device syncing at the same moment. The caller should just
    /// try again on the next scheduled/manual sync, not treat this as a
    /// hard failure.
    TooManyConflictRetries,
    /// `RemoteSyncConfig::endpoint` isn't a valid URL as configured,
    /// caught here rather than at `S3Client` construction since it's the
    /// one failure mode that's a setup mistake rather than a runtime
    /// network/bucket problem.
    InvalidEndpoint(String),
    /// A concurrent push/pull task panicked or was cancelled. Carries
    /// the joined task's own `Display` output, since `tokio::task::JoinError`
    /// itself doesn't implement `Clone`/`Serialize` and this crate's error
    /// types are kept simple string-carrying variants throughout.
    TaskJoin(String),
    /// The caller's cancellation flag was set mid-pass. See
    /// `run_sync`'s `cancel` parameter. Treated exactly like a hard
    /// process kill mid-sync (see the "what happens if the app closes
    /// mid-sync" behavior this mirrors on purpose): no manifest write
    /// happens, whatever already uploaded stays in the bucket as a
    /// harmless not-yet-referenced blob, and the next sync just resumes
    /// from a fresh diff. Not surfaced to the user as an error.
    Cancelled,
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::S3(e) => write!(f, "{e}"),
            Self::Manifest(e) => write!(f, "{e}"),
            Self::Db(e) => write!(f, "{e}"),
            Self::TooManyConflictRetries => {
                write!(f, "sync manifest kept changing underneath us; try again")
            }
            Self::InvalidEndpoint(endpoint) => write!(f, "invalid bucket endpoint URL: {endpoint}"),
            Self::TaskJoin(msg) => write!(f, "a concurrent sync task failed: {msg}"),
            Self::Cancelled => write!(f, "sync was cancelled"),
        }
    }
}

impl std::error::Error for SyncError {}
impl From<S3Error> for SyncError {
    fn from(e: S3Error) -> Self {
        Self::S3(e)
    }
}
impl From<super::manifest::ManifestError> for SyncError {
    fn from(e: super::manifest::ManifestError) -> Self {
        Self::Manifest(e)
    }
}
impl From<rusqlite::Error> for SyncError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Db(e)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct SyncOutcome {
    pub pulled: usize,
    pub pushed: usize,
    pub tombstones_applied: usize,
    pub manifest_version: u64,
    /// Orphaned bucket blobs cleaned up by `bucket_gc::sweep_orphaned_blobs`
    /// this pass — usually 0. A sweep failure never fails the sync pass
    /// itself (see the call site), so this can undercount without that
    /// being surfaced as an error.
    pub orphans_deleted: usize,
}

/// Which of the six pull/push phases a [`SyncProgress`] update is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncPhase {
    PullSources,
    PullCategories,
    PullArticles,
    PushSources,
    PushCategories,
    PushArticles,
}

impl SyncPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PullSources => "pull_sources",
            Self::PullCategories => "pull_categories",
            Self::PullArticles => "pull_articles",
            Self::PushSources => "push_sources",
            Self::PushCategories => "push_categories",
            Self::PushArticles => "push_articles",
        }
    }
}

/// One incremental progress update from a concurrent push/pull phase.
/// `total` is fixed for the whole phase (computed once, up front, from
/// how many rows actually need pushing/pulling), `completed` increases
/// by one every time a concurrent task finishes, regardless of the order
/// concurrent tasks happen to complete in. Reported via a plain callback
/// (`&mut dyn FnMut`, not a channel) since emitting a Tauri event is
/// itself synchronous — see `remote_sync::orchestrate`.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct SyncProgress {
    pub phase: SyncPhase,
    pub completed: usize,
    pub total: usize,
}

/// Runs one full sync pass against `conn`, retrying up to
/// `MAX_CONFLICT_RETRIES` times if another device's manifest write races
/// this one. `data_dir` is only used to delete a tombstoned article's
/// `content/<id>/` directory and hero thumbnail — everything else is
/// pure DB + network.
pub async fn run_sync(
    conn: &Connection,
    client: &S3Client,
    data_dir: &Path,
    now: DateTime<Utc>,
    cancel: &Arc<AtomicBool>,
    mut on_progress: impl FnMut(SyncProgress),
) -> Result<SyncOutcome, SyncError> {
    for _ in 0..MAX_CONFLICT_RETRIES {
        if cancel.load(Ordering::Relaxed) {
            return Err(SyncError::Cancelled);
        }
        if let Some(outcome) =
            try_sync_once(conn, client, data_dir, now, cancel, &mut on_progress).await?
        {
            return Ok(outcome);
        }
    }
    Err(SyncError::TooManyConflictRetries)
}

pub fn client_from_config(config: &RemoteSyncConfig) -> Result<S3Client, SyncError> {
    let endpoint = config
        .endpoint
        .parse()
        .map_err(|_| SyncError::InvalidEndpoint(config.endpoint.clone()))?;
    Ok(S3Client::new(&BucketConfig {
        endpoint,
        bucket_name: config.bucket_name.clone(),
        region: config.region.clone(),
        use_path_style: config.use_path_style,
        access_key: config.access_key.clone(),
        secret_key: config.secret_key.clone(),
    })?)
}

/// One attempt: `Ok(Some(outcome))` on success, `Ok(None)` if the
/// manifest write lost a race and the caller should retry from scratch.
async fn try_sync_once(
    conn: &Connection,
    client: &S3Client,
    data_dir: &Path,
    now: DateTime<Utc>,
    cancel: &Arc<AtomicBool>,
    on_progress: &mut dyn FnMut(SyncProgress),
) -> Result<Option<SyncOutcome>, SyncError> {
    let (mut manifest, precondition) = match client.get_object(MANIFEST_KEY).await? {
        Some((bytes, etag)) => (Manifest::from_gz_bytes(&bytes)?, Some(etag)),
        None => (Manifest::default(), None),
    };
    manifest.purge_expired_tombstones(now, TOMBSTONE_RETENTION);

    let tombstones_applied = apply_tombstones_locally(conn, data_dir, &manifest).await?;

    let mut pulled = 0;
    pulled += pull_sources(conn, client, &manifest, now, cancel, on_progress).await?;
    pulled += pull_categories(conn, client, &manifest, now, cancel, on_progress).await?;
    pulled += pull_articles(conn, client, &manifest, now, cancel, on_progress).await?;

    resolve_source_collisions(conn, now)?;
    resolve_category_collisions(conn, now)?;
    resolve_article_collisions(conn, now)?;

    if cancel.load(Ordering::Relaxed) {
        return Err(SyncError::Cancelled);
    }

    let mut pushed = 0;
    pushed += push_sources(conn, client, &mut manifest, cancel, on_progress).await?;
    pushed += push_categories(conn, client, &mut manifest, cancel, on_progress).await?;
    pushed += push_articles(conn, client, &mut manifest, data_dir, cancel, on_progress).await?;

    merge_local_tombstones_into_manifest(conn, &mut manifest)?;

    manifest.version += 1;
    let bytes = manifest.to_gz_bytes();
    match client
        .put_object_if_match(MANIFEST_KEY, bytes, precondition.as_deref())
        .await?
    {
        PutOutcome::Written { .. } => {
            // A GC failure never fails the sync pass that just succeeded
            // — cleanup is a nice-to-have, not a correctness requirement,
            // and the next successful pass gets another chance at it.
            // Cancellation is the one exception: propagate it exactly
            // like every other phase does, since "the user asked this to
            // stop" shouldn't be swallowed just because it happened
            // during cleanup instead of the main pass.
            let orphans_deleted = match super::bucket_gc::sweep_orphaned_blobs(
                client, &manifest, cancel,
            )
            .await
            {
                Ok(count) => count,
                Err(SyncError::Cancelled) => return Err(SyncError::Cancelled),
                Err(err) => {
                    tracing::warn!(%err, "orphaned blob sweep failed; sync itself still succeeded");
                    0
                }
            };
            Ok(Some(SyncOutcome {
                pulled,
                pushed,
                tombstones_applied,
                manifest_version: manifest.version,
                orphans_deleted,
            }))
        }
        PutOutcome::Conflict => Ok(None),
    }
}

fn gz_json<T: serde::Serialize>(value: &T) -> Vec<u8> {
    // Mirrors `Manifest::to_gz_bytes` — see that doc comment for why
    // `flate2`'s pure-Rust backend, not re-derived here as a shared
    // helper to keep `manifest.rs` self-contained and easy to reason
    // about in isolation.
    use std::io::Write as _;
    let json = serde_json::to_vec(value).expect("row serialization cannot fail");
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    encoder
        .write_all(&json)
        .expect("writing to an in-memory Vec<u8> should never fail");
    encoder
        .finish()
        .expect("finishing an in-memory gzip stream should never fail")
}

fn from_gz_json<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Option<T> {
    use std::io::Read as _;
    let mut decoder = flate2::read::GzDecoder::new(bytes);
    let mut json = Vec::new();
    decoder.read_to_end(&mut json).ok()?;
    serde_json::from_slice(&json).ok()
}

fn content_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Parses an RFC 3339 timestamp for ordering comparisons. Anything that
/// fails to parse (shouldn't happen — every writer of these columns goes
/// through `chrono::Utc::now().to_rfc3339()`) sorts as the oldest
/// possible instant, so a malformed value can never incorrectly win a
/// last-write-wins comparison against real data.
fn parse_ts(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or(DateTime::<Utc>::MIN_UTC)
}

async fn apply_tombstones_locally(
    conn: &Connection,
    data_dir: &Path,
    manifest: &Manifest,
) -> Result<usize, SyncError> {
    let mut applied = 0;
    for tombstone in &manifest.tombstones {
        let hero_image_path = sync_rows::apply_tombstone(
            conn,
            tombstone.entity_type.as_str(),
            &tombstone.entity_id,
            &tombstone.deleted_at,
        )?;
        if let Some(hero_image_path) = hero_image_path {
            let _ = tokio::fs::remove_dir_all(data_dir.join("content").join(&tombstone.entity_id))
                .await;
            if let Some(hero) = hero_image_path {
                let _ = tokio::fs::remove_file(data_dir.join(hero)).await;
            }
        }
        applied += 1;
    }
    Ok(applied)
}

pub(super) fn blob_key(entity_type: EntityType, id: &str) -> String {
    format!(
        "legere-sync/blobs/{}/{id}/meta.json.gz",
        entity_type.plural_str()
    )
}

/// Whether `entry` describes a newer version of the entity than what's
/// in `local`, i.e. this device should pull it. Missing from `local`
/// entirely also counts as needing a pull.
fn needs_pull(local: Option<&str>, entry_updated_at: &str) -> bool {
    match local {
        None => true,
        Some(local_updated_at) => parse_ts(local_updated_at) < parse_ts(entry_updated_at),
    }
}

/// Whether a local row's `updated_at` is strictly newer than what the
/// manifest currently has for this id (`None` if it's not there at
/// all yet) — i.e. this device should push it. Symmetric with
/// `needs_pull` above, just comparing in the opposite direction.
fn needs_push(local_updated_at: &str, existing_updated_at: Option<&str>) -> bool {
    match existing_updated_at {
        None => true,
        Some(existing) => parse_ts(local_updated_at) > parse_ts(existing),
    }
}

/// Whether a manifest entry's incoming id collides with a *different*
/// local id sharing the same `conflict_key`. The schema-level `UNIQUE`
/// constraints on `articles.link`/`categories.name`/`sources.feed_url`
/// mean this must be resolved before any insert is attempted, not after:
/// a naive pull-then-reconcile ordering would hit the UNIQUE constraint
/// immediately, since both rows would transiently exist at once.
fn colliding_local_id<'a>(
    entry: &ManifestEntry,
    local_by_conflict_key: &HashMap<&'a str, &'a str>,
) -> Option<&'a str> {
    local_by_conflict_key
        .get(entry.conflict_key.as_str())
        .copied()
        .filter(|&local_id| local_id != entry.id)
}

/// Generic concurrent "fetch every plain (non-collision) id" phase
/// shared by the three `pull_*` functions: downloads each blob with up
/// to `SYNC_CONCURRENCY` requests in flight, applying each one (via
/// `apply_one`, which *does* touch the DB — safe here since it only
/// ever runs on this function's own task after a fetch completes, never
/// inside the spawned download task itself) as it completes. Returns how
/// many were actually applied (a fetch that 404s or fails to deserialize
/// is skipped, not an error — matches every pull path's existing
/// tolerance for a manifest entry whose blob went missing).
#[allow(clippy::too_many_arguments)]
async fn pull_concurrently<T>(
    conn: &Connection,
    client: &S3Client,
    ids: Vec<String>,
    entity_type: EntityType,
    phase: SyncPhase,
    cancel: &Arc<AtomicBool>,
    on_progress: &mut dyn FnMut(SyncProgress),
    apply_one: impl Fn(&Connection, T) -> rusqlite::Result<()>,
) -> Result<usize, SyncError>
where
    T: serde::de::DeserializeOwned + Send + 'static,
{
    let total = ids.len();
    if total == 0 {
        return Ok(0);
    }

    let mut join_set = tokio::task::JoinSet::new();
    let mut ids_iter = ids.into_iter();
    let mut applied = 0usize;
    let mut completed = 0usize;

    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(SyncError::Cancelled);
        }
        while join_set.len() < SYNC_CONCURRENCY {
            let Some(id) = ids_iter.next() else { break };
            let client = client.clone();
            join_set.spawn(async move { client.get_object(&blob_key(entity_type, &id)).await });
        }
        let Some(joined) = join_set.join_next().await else {
            break;
        };
        let fetch_result = joined.map_err(|e| SyncError::TaskJoin(e.to_string()))??;
        completed += 1;
        on_progress(SyncProgress {
            phase,
            completed,
            total,
        });
        let Some((bytes, _)) = fetch_result else {
            continue;
        };
        let Some(row) = from_gz_json::<T>(&bytes) else {
            continue;
        };
        apply_one(conn, row)?;
        applied += 1;
    }

    Ok(applied)
}

async fn pull_sources(
    conn: &Connection,
    client: &S3Client,
    manifest: &Manifest,
    now: DateTime<Utc>,
    cancel: &Arc<AtomicBool>,
    on_progress: &mut dyn FnMut(SyncProgress),
) -> Result<usize, SyncError> {
    let local_rows = sync_rows::list_sources_for_sync(conn)?;
    let local_by_id: HashMap<&str, &SyncSourceRow> =
        local_rows.iter().map(|r| (r.id.as_str(), r)).collect();
    let local_by_key: HashMap<&str, &str> = local_rows
        .iter()
        .map(|r| (r.feed_url.as_str(), r.id.as_str()))
        .collect();

    let mut pulled = 0;
    let mut to_fetch = Vec::new();
    for entry in manifest
        .live_entries()
        .filter(|e| e.entity_type == EntityType::Source)
    {
        if let Some(loser_id) = colliding_local_id(entry, &local_by_key) {
            let local = local_by_id[loser_id];
            let local_candidate = Candidate {
                id: loser_id.to_string(),
                created_at: local.created_at.clone(),
            };
            let remote_candidate = Candidate {
                id: entry.id.clone(),
                created_at: entry.created_at.clone(),
            };
            let (winner, _loser) = conflict::pick_winner(&local_candidate, &remote_candidate);
            if winner.id == entry.id {
                // Remote wins: drop the local competitor, then fall
                // through to a normal pull of the remote row below.
                sync_rows::apply_tombstone(conn, "source", loser_id, &now.to_rfc3339())?;
            } else {
                // Local wins: the remote id must never be materialized;
                // tombstone it directly without ever inserting it.
                sync_rows::apply_tombstone(conn, "source", &entry.id, &now.to_rfc3339())?;
                continue;
            }
        } else if !needs_pull(
            local_by_id
                .get(entry.id.as_str())
                .map(|r| r.updated_at.as_str()),
            &entry.updated_at,
        ) {
            continue;
        }
        to_fetch.push(entry.id.clone());
    }

    pulled += pull_concurrently::<SyncSourceRow>(
        conn,
        client,
        to_fetch,
        EntityType::Source,
        SyncPhase::PullSources,
        cancel,
        on_progress,
        |conn, row| sync_rows::upsert_synced_source(conn, &row),
    )
    .await?;
    Ok(pulled)
}

async fn pull_categories(
    conn: &Connection,
    client: &S3Client,
    manifest: &Manifest,
    now: DateTime<Utc>,
    cancel: &Arc<AtomicBool>,
    on_progress: &mut dyn FnMut(SyncProgress),
) -> Result<usize, SyncError> {
    let local_rows = sync_rows::list_categories_for_sync(conn)?;
    let local_by_id: HashMap<&str, &SyncCategoryRow> =
        local_rows.iter().map(|r| (r.id.as_str(), r)).collect();
    let local_by_key: HashMap<&str, &str> = local_rows
        .iter()
        .map(|r| (r.name.as_str(), r.id.as_str()))
        .collect();

    let mut to_fetch = Vec::new();
    for entry in manifest
        .live_entries()
        .filter(|e| e.entity_type == EntityType::Category)
    {
        if let Some(loser_id) = colliding_local_id(entry, &local_by_key) {
            let local = local_by_id[loser_id];
            let local_candidate = Candidate {
                id: loser_id.to_string(),
                created_at: local.created_at.clone(),
            };
            let remote_candidate = Candidate {
                id: entry.id.clone(),
                created_at: entry.created_at.clone(),
            };
            let (winner, _loser) = conflict::pick_winner(&local_candidate, &remote_candidate);
            if winner.id == entry.id {
                sync_rows::apply_tombstone(conn, "category", loser_id, &now.to_rfc3339())?;
            } else {
                sync_rows::apply_tombstone(conn, "category", &entry.id, &now.to_rfc3339())?;
                continue;
            }
        } else if !needs_pull(
            local_by_id
                .get(entry.id.as_str())
                .map(|r| r.updated_at.as_str()),
            &entry.updated_at,
        ) {
            continue;
        }
        to_fetch.push(entry.id.clone());
    }

    pull_concurrently::<SyncCategoryRow>(
        conn,
        client,
        to_fetch,
        EntityType::Category,
        SyncPhase::PullCategories,
        cancel,
        on_progress,
        |conn, row| sync_rows::upsert_synced_category(conn, &row),
    )
    .await
}

async fn pull_articles(
    conn: &Connection,
    client: &S3Client,
    manifest: &Manifest,
    now: DateTime<Utc>,
    cancel: &Arc<AtomicBool>,
    on_progress: &mut dyn FnMut(SyncProgress),
) -> Result<usize, SyncError> {
    let local_rows = sync_rows::list_articles_for_sync(conn)?;
    let local_by_id: HashMap<&str, &SyncArticleRow> =
        local_rows.iter().map(|r| (r.id.as_str(), r)).collect();
    let local_by_key: HashMap<&str, &str> = local_rows
        .iter()
        .map(|r| (r.link.as_str(), r.id.as_str()))
        .collect();

    let mut pulled = 0;
    let mut to_fetch = Vec::new();
    for entry in manifest
        .live_entries()
        .filter(|e| e.entity_type == EntityType::Article)
    {
        // A same-link collision against a different local id needs its
        // winner decided (and the loser's mutable state - tags, reading
        // progress, favorited - merged into whichever survives) before
        // any insert is attempted; see colliding_local_id's doc comment.
        // Handled inline/sequentially here (rare in practice) rather
        // than folded into the concurrent phase below, which only ever
        // handles the plain, no-collision case.
        if let Some(loser_id) = colliding_local_id(entry, &local_by_key) {
            let local = local_by_id[loser_id].clone();
            let local_candidate = Candidate {
                id: loser_id.to_string(),
                created_at: local.fetched_at.clone(),
            };
            let remote_candidate = Candidate {
                id: entry.id.clone(),
                created_at: entry.created_at.clone(),
            };
            let (winner, _loser) = conflict::pick_winner(&local_candidate, &remote_candidate);

            let Some((bytes, _)) = client
                .get_object(&blob_key(EntityType::Article, &entry.id))
                .await?
            else {
                continue;
            };
            let Some(remote_row) = from_gz_json::<SyncArticleRow>(&bytes) else {
                continue;
            };

            let merged = conflict::merge_article_state(
                &ArticleMutableState {
                    tags: remote_row.tags.clone(),
                    reading_progress: remote_row.reading_progress,
                    favorited: remote_row.favorited,
                },
                &ArticleMutableState {
                    tags: local.tags.clone(),
                    reading_progress: local.reading_progress,
                    favorited: local.favorited,
                },
            );

            if winner.id == entry.id {
                // Remote wins: tombstone the local loser, materialize
                // the remote row with the merged mutable state. Bumping
                // `updated_at` to `now` is required, not cosmetic: the
                // merge itself is a new change this device must push
                // back out on its next pass (see push_articles's
                // `needs_push`). A third device that already had a
                // local copy of the same link would otherwise silently
                // never see this merge, since remote_row's own
                // `updated_at` predates it and a plain overwrite would
                // look unchanged to `needs_push`.
                sync_rows::apply_tombstone(conn, "article", loser_id, &now.to_rfc3339())?;
                let mut row = remote_row;
                row.tags = merged.tags;
                row.reading_progress = merged.reading_progress;
                row.favorited = merged.favorited;
                row.updated_at = now.to_rfc3339();
                sync_rows::upsert_synced_article(conn, &row)?;
                pulled += 1;
            } else {
                // Local wins: the remote id must never be materialized.
                // Still fold the remote row's mutable state into the
                // local survivor so a harmless capture race never
                // silently drops real user actions taken on the losing
                // copy before the race was noticed.
                sync_rows::apply_tombstone(conn, "article", &entry.id, &now.to_rfc3339())?;
                let mut updated_local = local;
                updated_local.tags = merged.tags;
                updated_local.reading_progress = merged.reading_progress;
                updated_local.favorited = merged.favorited;
                updated_local.updated_at = now.to_rfc3339();
                sync_rows::upsert_synced_article(conn, &updated_local)?;
            }
            continue;
        }

        if !needs_pull(
            local_by_id
                .get(entry.id.as_str())
                .map(|r| r.updated_at.as_str()),
            &entry.updated_at,
        ) {
            continue;
        }
        to_fetch.push(entry.id.clone());
    }

    pulled += pull_concurrently::<SyncArticleRow>(
        conn,
        client,
        to_fetch,
        EntityType::Article,
        SyncPhase::PullArticles,
        cancel,
        on_progress,
        |conn, row| sync_rows::upsert_synced_article(conn, &row),
    )
    .await?;
    Ok(pulled)
}

/// Generic concurrent "push what changed" loop shared by
/// `push_sources`/`push_categories`/`push_articles`: uploads every row in
/// `to_push` via `upload_one` with up to `SYNC_CONCURRENCY` requests in
/// flight at once, reporting one [`SyncProgress`] per completed upload.
/// `upload_one` does the blob serialization/upload only — no DB access —
/// so it can run inside a `tokio::spawn`'d task, which requires `'static`
/// (hence `to_push` is consumed by value, and `client` is cloned per
/// task rather than borrowed — see `S3Client`'s `Clone` impl doc comment).
async fn push_concurrently<T, F, Fut>(
    client: &S3Client,
    to_push: Vec<T>,
    phase: SyncPhase,
    cancel: &Arc<AtomicBool>,
    on_progress: &mut dyn FnMut(SyncProgress),
    upload_one: F,
) -> Result<Vec<ManifestEntry>, SyncError>
where
    T: Send + 'static,
    F: Fn(S3Client, T) -> Fut,
    Fut: std::future::Future<Output = Result<ManifestEntry, S3Error>> + Send + 'static,
{
    let total = to_push.len();
    let mut new_entries = Vec::with_capacity(total);
    if total == 0 {
        return Ok(new_entries);
    }

    let mut join_set = tokio::task::JoinSet::new();
    let mut rows_iter = to_push.into_iter();
    let mut completed = 0usize;

    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(SyncError::Cancelled);
        }
        while join_set.len() < SYNC_CONCURRENCY {
            let Some(row) = rows_iter.next() else { break };
            let client = client.clone();
            join_set.spawn(upload_one(client, row));
        }
        let Some(joined) = join_set.join_next().await else {
            break;
        };
        let entry = joined.map_err(|e| SyncError::TaskJoin(e.to_string()))??;
        new_entries.push(entry);
        completed += 1;
        on_progress(SyncProgress {
            phase,
            completed,
            total,
        });
    }

    Ok(new_entries)
}

async fn push_sources(
    conn: &Connection,
    client: &S3Client,
    manifest: &mut Manifest,
    cancel: &Arc<AtomicBool>,
    on_progress: &mut dyn FnMut(SyncProgress),
) -> Result<usize, SyncError> {
    let existing: HashMap<String, String> = manifest
        .entries
        .iter()
        .filter(|e| e.entity_type == EntityType::Source)
        .map(|e| (e.id.clone(), e.updated_at.clone()))
        .collect();
    let to_push: Vec<SyncSourceRow> = sync_rows::list_sources_for_sync(conn)?
        .into_iter()
        .filter(|row| needs_push(&row.updated_at, existing.get(&row.id).map(|s| s.as_str())))
        .collect();

    let new_entries = push_concurrently(
        client,
        to_push,
        SyncPhase::PushSources,
        cancel,
        on_progress,
        |client, row| async move {
            let bytes = gz_json(&row);
            let hash = content_hash(&bytes);
            client
                .put_object(&blob_key(EntityType::Source, &row.id), bytes)
                .await?;
            Ok(ManifestEntry {
                id: row.id.clone(),
                entity_type: EntityType::Source,
                conflict_key: row.feed_url.clone(),
                updated_at: row.updated_at.clone(),
                created_at: row.created_at.clone(),
                content_hash: hash,
            })
        },
    )
    .await?;

    let pushed = new_entries.len();
    replace_entries(manifest, EntityType::Source, new_entries);
    Ok(pushed)
}

async fn push_categories(
    conn: &Connection,
    client: &S3Client,
    manifest: &mut Manifest,
    cancel: &Arc<AtomicBool>,
    on_progress: &mut dyn FnMut(SyncProgress),
) -> Result<usize, SyncError> {
    let existing: HashMap<String, String> = manifest
        .entries
        .iter()
        .filter(|e| e.entity_type == EntityType::Category)
        .map(|e| (e.id.clone(), e.updated_at.clone()))
        .collect();
    let to_push: Vec<SyncCategoryRow> = sync_rows::list_categories_for_sync(conn)?
        .into_iter()
        .filter(|row| needs_push(&row.updated_at, existing.get(&row.id).map(|s| s.as_str())))
        .collect();

    let new_entries = push_concurrently(
        client,
        to_push,
        SyncPhase::PushCategories,
        cancel,
        on_progress,
        |client, row| async move {
            let bytes = gz_json(&row);
            let hash = content_hash(&bytes);
            client
                .put_object(&blob_key(EntityType::Category, &row.id), bytes)
                .await?;
            Ok(ManifestEntry {
                id: row.id.clone(),
                entity_type: EntityType::Category,
                conflict_key: row.name.to_lowercase(),
                updated_at: row.updated_at.clone(),
                created_at: row.created_at.clone(),
                content_hash: hash,
            })
        },
    )
    .await?;

    let pushed = new_entries.len();
    replace_entries(manifest, EntityType::Category, new_entries);
    Ok(pushed)
}

/// Uploads an article's images — its hero thumbnail (if any) and every
/// file under `content/<id>/` — to the bucket. Only ever called for an
/// article this device is pushing for the *first* time (see
/// `push_articles`'s `is_new` check): a given id's images are immutable
/// after capture (the one path that changes an article's content,
/// re-capture, produces a brand-new id — see ARCHITECTURE.md's capture
/// pipeline), so re-uploading them on every later metadata-only push
/// (a tag edit, a favorite toggle, ...) would be pure waste. A missing
/// local file (already evicted, or this row has no images at all) is
/// skipped, not an error.
async fn upload_article_images(
    client: &S3Client,
    data_dir: &Path,
    id: &str,
    hero_image_path: Option<&str>,
) -> Result<(), S3Error> {
    if let Some(hero) = hero_image_path
        && let Ok(bytes) = tokio::fs::read(data_dir.join(hero)).await
    {
        client
            .put_object(&format!("legere-sync/blobs/articles/{id}/hero.jpg"), bytes)
            .await?;
    }

    let content_dir = data_dir.join("content").join(id);
    for file in collect_files_recursively(&content_dir).await {
        let Ok(relative) = file.strip_prefix(&content_dir) else {
            continue;
        };
        let Some(relative_str) = relative.to_str() else {
            continue;
        };
        let Ok(bytes) = tokio::fs::read(&file).await else {
            continue;
        };
        client
            .put_object(
                &format!("legere-sync/blobs/articles/{id}/images/{relative_str}"),
                bytes,
            )
            .await?;
    }

    Ok(())
}

/// Every regular file under `dir`, recursively — `content/<id>/` can
/// nest arbitrarily deep (mirroring the source page's own URL path, see
/// `urlx`'s path mapping). Returns an empty list rather than an error for
/// a directory that doesn't exist (an article with no images at all).
async fn collect_files_recursively(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let Ok(mut entries) = tokio::fs::read_dir(&current).await else {
            continue;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let Ok(file_type) = entry.file_type().await else {
                continue;
            };
            if file_type.is_dir() {
                pending.push(entry.path());
            } else if file_type.is_file() {
                files.push(entry.path());
            }
        }
    }
    files
}

async fn push_articles(
    conn: &Connection,
    client: &S3Client,
    manifest: &mut Manifest,
    data_dir: &Path,
    cancel: &Arc<AtomicBool>,
    on_progress: &mut dyn FnMut(SyncProgress),
) -> Result<usize, SyncError> {
    let existing: HashMap<String, String> = manifest
        .entries
        .iter()
        .filter(|e| e.entity_type == EntityType::Article)
        .map(|e| (e.id.clone(), e.updated_at.clone()))
        .collect();
    let to_push: Vec<(SyncArticleRow, bool)> = sync_rows::list_articles_for_sync(conn)?
        .into_iter()
        .filter(|row| needs_push(&row.updated_at, existing.get(&row.id).map(|s| s.as_str())))
        .map(|row| {
            let is_new = !existing.contains_key(&row.id);
            (row, is_new)
        })
        .collect();

    let data_dir = data_dir.to_path_buf();
    let new_entries = push_concurrently(
        client,
        to_push,
        SyncPhase::PushArticles,
        cancel,
        on_progress,
        move |client, (row, is_new)| {
            let data_dir = data_dir.clone();
            async move {
                if is_new {
                    upload_article_images(
                        &client,
                        &data_dir,
                        &row.id,
                        row.hero_image_path.as_deref(),
                    )
                    .await?;
                }
                let bytes = gz_json(&row);
                let hash = content_hash(&bytes);
                client
                    .put_object(&blob_key(EntityType::Article, &row.id), bytes)
                    .await?;
                Ok(ManifestEntry {
                    id: row.id.clone(),
                    entity_type: EntityType::Article,
                    conflict_key: row.link.clone(),
                    updated_at: row.updated_at.clone(),
                    created_at: row.fetched_at.clone(),
                    content_hash: hash,
                })
            }
        },
    )
    .await?;

    let pushed = new_entries.len();
    replace_entries(manifest, EntityType::Article, new_entries);
    Ok(pushed)
}

fn replace_entries(
    manifest: &mut Manifest,
    entity_type: EntityType,
    new_entries: Vec<ManifestEntry>,
) {
    if new_entries.is_empty() {
        return;
    }
    let pushed_ids: std::collections::HashSet<&str> =
        new_entries.iter().map(|e| e.id.as_str()).collect();
    manifest
        .entries
        .retain(|e| !(e.entity_type == entity_type && pushed_ids.contains(e.id.as_str())));
    manifest.entries.extend(new_entries);
}

fn resolve_source_collisions(conn: &Connection, now: DateTime<Utc>) -> Result<(), SyncError> {
    let rows = sync_rows::list_sources_for_sync(conn)?;
    for loser_id in collision_losers(&rows, |r| r.feed_url.clone(), |r| &r.id, |r| &r.created_at) {
        sync_rows::apply_tombstone(conn, "source", &loser_id, &now.to_rfc3339())?;
    }
    Ok(())
}

fn resolve_category_collisions(conn: &Connection, now: DateTime<Utc>) -> Result<(), SyncError> {
    let rows = sync_rows::list_categories_for_sync(conn)?;
    for loser_id in collision_losers(
        &rows,
        |r| r.name.to_lowercase(),
        |r| &r.id,
        |r| &r.created_at,
    ) {
        sync_rows::apply_tombstone(conn, "category", &loser_id, &now.to_rfc3339())?;
    }
    Ok(())
}

fn resolve_article_collisions(conn: &Connection, now: DateTime<Utc>) -> Result<(), SyncError> {
    let rows = sync_rows::list_articles_for_sync(conn)?;
    let mut by_link: HashMap<&str, Vec<&SyncArticleRow>> = HashMap::new();
    for row in &rows {
        by_link.entry(row.link.as_str()).or_default().push(row);
    }

    for group in by_link.values() {
        if group.len() < 2 {
            continue;
        }
        let mut winner_row = group[0];
        for candidate_row in &group[1..] {
            let winner_candidate = Candidate {
                id: winner_row.id.clone(),
                created_at: winner_row.fetched_at.clone(),
            };
            let candidate = Candidate {
                id: candidate_row.id.clone(),
                created_at: candidate_row.fetched_at.clone(),
            };
            let (winner, loser) = conflict::pick_winner(&winner_candidate, &candidate);
            let loser_row = if loser.id == winner_row.id {
                winner_row
            } else {
                candidate_row
            };
            winner_row = if winner.id == winner_row.id {
                winner_row
            } else {
                candidate_row
            };

            let merged = conflict::merge_article_state(
                &ArticleMutableState {
                    tags: winner_row.tags.clone(),
                    reading_progress: winner_row.reading_progress,
                    favorited: winner_row.favorited,
                },
                &ArticleMutableState {
                    tags: loser_row.tags.clone(),
                    reading_progress: loser_row.reading_progress,
                    favorited: loser_row.favorited,
                },
            );
            let mut updated_winner = winner_row.clone();
            updated_winner.tags = merged.tags;
            updated_winner.reading_progress = merged.reading_progress;
            updated_winner.favorited = merged.favorited;
            updated_winner.updated_at = now.to_rfc3339();
            sync_rows::upsert_synced_article(conn, &updated_winner)?;

            sync_rows::apply_tombstone(conn, "article", &loser_row.id, &now.to_rfc3339())?;
        }
    }
    Ok(())
}

/// Reduces a group of same-`conflict_key` rows to a single winner via
/// repeated pairwise `conflict::pick_winner`, returning every loser's id
/// (owned, so the caller can tombstone them after this borrow ends).
fn collision_losers<T>(
    rows: &[T],
    conflict_key: impl Fn(&T) -> String,
    id: impl Fn(&T) -> &str,
    created_at: impl Fn(&T) -> &str,
) -> Vec<String> {
    let mut by_key: HashMap<String, Vec<&T>> = HashMap::new();
    for row in rows {
        let key = conflict_key(row);
        if !key.is_empty() {
            by_key.entry(key).or_default().push(row);
        }
    }

    let mut losers = Vec::new();
    for group in by_key.values() {
        if group.len() < 2 {
            continue;
        }
        let mut winner = Candidate {
            id: id(group[0]).to_string(),
            created_at: created_at(group[0]).to_string(),
        };
        for row in &group[1..] {
            let candidate = Candidate {
                id: id(row).to_string(),
                created_at: created_at(row).to_string(),
            };
            let (w, l) = conflict::pick_winner(&winner, &candidate);
            losers.push(l.id.clone());
            winner = w.clone();
        }
    }
    losers
}

fn merge_local_tombstones_into_manifest(
    conn: &Connection,
    manifest: &mut Manifest,
) -> Result<(), SyncError> {
    let local: Vec<LocalTombstone> = sync_rows::list_local_tombstones(conn)?;
    let mut by_id: HashMap<String, TombstoneEntry> = manifest
        .tombstones
        .drain(..)
        .map(|t| (t.entity_id.clone(), t))
        .collect();

    for tombstone in local {
        let entity_type = match tombstone.entity_type.as_str() {
            "article" => EntityType::Article,
            "category" => EntityType::Category,
            "source" => EntityType::Source,
            _ => continue,
        };
        by_id
            .entry(tombstone.entity_id.clone())
            .and_modify(|existing| {
                // Keep whichever timestamp is older — the original
                // tombstoning event, if the two ever disagree (should be
                // rare: normally only one device ever originates a given
                // tombstone).
                if parse_ts(&tombstone.deleted_at) < parse_ts(&existing.deleted_at) {
                    existing.deleted_at = tombstone.deleted_at.clone();
                }
            })
            .or_insert(TombstoneEntry {
                entity_type,
                entity_id: tombstone.entity_id,
                deleted_at: tombstone.deleted_at,
            });
    }

    manifest.tombstones = by_id.into_values().collect();
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use axum::Router;
    use axum::body::Bytes;
    use axum::extract::{Path as AxumPath, State};
    use axum::http::{HeaderMap, StatusCode};
    use axum::response::IntoResponse;
    use axum::routing::get;
    use rusqlite::Connection;

    use super::*;
    use crate::capture::LocalCaptureOutput;
    use crate::db::sync_rows;

    fn no_cancel() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    #[tokio::test]
    async fn images_upload_once_for_a_new_article_and_never_again_on_metadata_only_pushes() {
        let (base_url, store) = spawn_mock_s3().await;
        let client = test_client(&base_url);
        let data_dir = tempfile::tempdir().unwrap();

        let device = migrated_conn();
        std::fs::create_dir_all(data_dir.path().join("content/art-1/sub")).unwrap();
        std::fs::write(
            data_dir.path().join("content/art-1/sub/image.jpg"),
            b"fake image",
        )
        .unwrap();
        std::fs::create_dir_all(data_dir.path().join("media")).unwrap();
        std::fs::write(data_dir.path().join("media/art-1.jpg"), b"fake hero").unwrap();

        let mut output = sample_output("https://example.com/a", "Hello");
        output.hero_image_path = Some("media/art-1.jpg".to_string());
        crate::db::queries::insert_captured_article(&device, "art-1", None, "direct", &output, &[])
            .unwrap();

        run_sync(
            &device,
            &client,
            data_dir.path(),
            Utc::now(),
            &no_cancel(),
            |_| {},
        )
        .await
        .unwrap();

        let hero_key = "legere-sync/blobs/articles/art-1/hero.jpg";
        let image_key = "legere-sync/blobs/articles/art-1/images/sub/image.jpg";
        let meta_key = "legere-sync/blobs/articles/art-1/meta.json.gz";
        {
            let guard = store.lock().unwrap();
            assert_eq!(guard.get(hero_key).unwrap().2, 1, "hero must upload once");
            assert_eq!(
                guard.get(image_key).unwrap().2,
                1,
                "content image must upload once"
            );
            assert_eq!(guard.get(meta_key).unwrap().2, 1);
        }

        // A metadata-only change (no re-capture, no new id) must push the
        // metadata blob again but never touch the already-uploaded images.
        device
            .execute(
                "UPDATE articles SET favorited = 1, updated_at = '2030-01-01T00:00:00Z' WHERE id = 'art-1'",
                [],
            )
            .unwrap();
        run_sync(
            &device,
            &client,
            data_dir.path(),
            Utc::now(),
            &no_cancel(),
            |_| {},
        )
        .await
        .unwrap();

        let guard = store.lock().unwrap();
        assert_eq!(
            guard.get(hero_key).unwrap().2,
            1,
            "hero must not be re-uploaded"
        );
        assert_eq!(
            guard.get(image_key).unwrap().2,
            1,
            "content image must not be re-uploaded"
        );
        assert_eq!(
            guard.get(meta_key).unwrap().2,
            2,
            "metadata must still be pushed again"
        );
    }

    #[tokio::test]
    async fn cancelling_mid_push_stops_the_pass_and_writes_no_manifest() {
        let (base_url, store) = spawn_mock_s3().await;
        let client = test_client(&base_url);
        let data_dir = tempfile::tempdir().unwrap();

        let device = migrated_conn();
        for i in 0..25 {
            crate::db::queries::insert_captured_article(
                &device,
                &format!("art-{i}"),
                None,
                "direct",
                &sample_output(&format!("https://example.com/{i}"), &format!("Title {i}")),
                &[],
            )
            .unwrap();
        }

        // Flip the flag as soon as the first upload reports progress —
        // well before all 25 would finish, so this exercises a real
        // mid-flight stop, not a race against the pass already being done.
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_setter = cancel.clone();
        let result = run_sync(
            &device,
            &client,
            data_dir.path(),
            Utc::now(),
            &cancel,
            move |_: SyncProgress| {
                cancel_setter.store(true, Ordering::Relaxed);
            },
        )
        .await;

        assert!(
            matches!(result, Err(SyncError::Cancelled)),
            "expected Cancelled, got {result:?}"
        );
        assert!(
            store.lock().unwrap().get(MANIFEST_KEY).is_none(),
            "a cancelled pass must never write the manifest"
        );
    }

    #[test]
    fn blob_key_uses_the_correct_plural_for_every_entity_type() {
        // Regression test: `blob_key` used to naively append "s" to
        // `EntityType::as_str()`, which is correct for article/source
        // but produced the wrong path ("categorys") for category. Pins
        // the exact expected string per type so this can't silently
        // regress again — the mock-server-based integration tests below
        // don't catch this class of bug, since they just store whatever
        // key string they're given and round-trip it correctly either way.
        assert_eq!(
            blob_key(EntityType::Article, "abc"),
            "legere-sync/blobs/articles/abc/meta.json.gz"
        );
        assert_eq!(
            blob_key(EntityType::Category, "abc"),
            "legere-sync/blobs/categories/abc/meta.json.gz"
        );
        assert_eq!(
            blob_key(EntityType::Source, "abc"),
            "legere-sync/blobs/sources/abc/meta.json.gz"
        );
    }

    /// `(bytes, etag, put_count)` — `put_count` (times this exact key has
    /// ever been PUT) backs tests that assert something was uploaded
    /// exactly once, e.g. that an already-pushed article's images are
    /// never re-uploaded on a later metadata-only push.
    type Store = Arc<Mutex<HashMap<String, (Vec<u8>, String, usize)>>>;

    /// A minimal S3-compatible mock: enough `GET`/`PUT` (with
    /// `If-Match`/`If-None-Match` conditional handling)/`DELETE` on
    /// `/{bucket}/{*key}` for `S3Client` to talk to, with no signature
    /// verification at all — `S3Client` only needs a *server* that
    /// behaves like S3 for these tests, not one that actually validates
    /// the SigV4 query string, which rusty-s3's own upstream tests
    /// already cover.
    async fn spawn_mock_s3() -> (String, Store) {
        let store: Store = Arc::new(Mutex::new(HashMap::new()));

        async fn get_object(
            State(store): State<Store>,
            AxumPath((_bucket, key)): AxumPath<(String, String)>,
        ) -> impl axum::response::IntoResponse {
            match store.lock().unwrap().get(&key) {
                Some((bytes, etag, _put_count)) => {
                    (StatusCode::OK, [("etag", etag.clone())], bytes.clone()).into_response()
                }
                None => StatusCode::NOT_FOUND.into_response(),
            }
        }

        async fn put_object(
            State(store): State<Store>,
            AxumPath((_bucket, key)): AxumPath<(String, String)>,
            headers: HeaderMap,
            body: Bytes,
        ) -> impl axum::response::IntoResponse {
            let mut guard = store.lock().unwrap();
            let existing = guard.get(&key);
            let current_etag = existing.map(|(_, etag, _)| etag.clone());
            let put_count = existing.map_or(0, |(_, _, count)| *count);

            if let Some(expected) = headers.get("if-match").and_then(|v| v.to_str().ok())
                && current_etag.as_deref() != Some(expected)
            {
                return StatusCode::PRECONDITION_FAILED.into_response();
            }
            if headers.get("if-none-match").and_then(|v| v.to_str().ok()) == Some("*")
                && current_etag.is_some()
            {
                return StatusCode::PRECONDITION_FAILED.into_response();
            }

            let new_etag = format!("{:x}", md5_like_hash(&body));
            guard.insert(key, (body.to_vec(), new_etag.clone(), put_count + 1));
            (StatusCode::OK, [("etag", new_etag)]).into_response()
        }

        async fn delete_object(
            State(store): State<Store>,
            AxumPath((_bucket, key)): AxumPath<(String, String)>,
        ) -> impl axum::response::IntoResponse {
            store.lock().unwrap().remove(&key);
            StatusCode::NO_CONTENT
        }

        let app = Router::new()
            .route(
                "/{bucket}/{*key}",
                get(get_object).put(put_object).delete(delete_object),
            )
            .with_state(store.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock s3 server");
        let addr = listener.local_addr().expect("local_addr");
        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("mock s3 server should not fail");
        });

        (format!("http://localhost:{}", addr.port()), store)
    }

    /// A cheap, non-cryptographic stand-in for a real content hash —
    /// this mock only needs *some* value that changes when the body
    /// does, not a real digest (that's `engine::content_hash`'s job on
    /// the real client side, exercised separately).
    fn md5_like_hash(bytes: &[u8]) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hasher);
        hasher.finish()
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

    fn migrated_conn() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        crate::db::schema::migrate(&mut conn).expect("migrate");
        conn
    }

    fn sample_output(link: &str, title: &str) -> LocalCaptureOutput {
        LocalCaptureOutput {
            title: title.to_string(),
            link: link.to_string(),
            final_url: link.to_string(),
            excerpt: "excerpt".to_string(),
            content_html: format!("<p>{title}</p>"),
            published_at: None,
            read_time_min: 3,
            hero_image_path: None,
            extraction_confident: true,
        }
    }

    #[tokio::test]
    async fn pushing_many_articles_reports_monotonic_progress_and_pushes_all_of_them() {
        let (base_url, _store) = spawn_mock_s3().await;
        let client = test_client(&base_url);
        let data_dir = tempfile::tempdir().unwrap();

        let device = migrated_conn();
        const ARTICLE_COUNT: usize = 25;
        for i in 0..ARTICLE_COUNT {
            crate::db::queries::insert_captured_article(
                &device,
                &format!("art-{i}"),
                None,
                "direct",
                &sample_output(&format!("https://example.com/{i}"), &format!("Title {i}")),
                &[],
            )
            .unwrap();
        }

        let progress_log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log_for_callback = progress_log.clone();
        let outcome = run_sync(
            &device,
            &client,
            data_dir.path(),
            Utc::now(),
            &no_cancel(),
            move |p: SyncProgress| {
                log_for_callback.lock().unwrap().push(p);
            },
        )
        .await
        .unwrap();

        assert_eq!(outcome.pushed, ARTICLE_COUNT);

        let log = progress_log.lock().unwrap();
        let article_pushes: Vec<&SyncProgress> = log
            .iter()
            .filter(|p| p.phase == SyncPhase::PushArticles)
            .collect();
        assert_eq!(
            article_pushes.len(),
            ARTICLE_COUNT,
            "one progress update per pushed article, not batched or skipped"
        );
        assert!(
            article_pushes.iter().all(|p| p.total == ARTICLE_COUNT),
            "total must stay fixed for the whole phase"
        );
        let completed: Vec<usize> = article_pushes.iter().map(|p| p.completed).collect();
        let mut sorted = completed.clone();
        sorted.sort_unstable();
        assert_eq!(
            sorted,
            (1..=ARTICLE_COUNT).collect::<Vec<_>>(),
            "completed counts must cover 1..=total exactly once each, regardless of which \
             concurrent upload happens to finish in which order"
        );
    }

    #[tokio::test]
    async fn a_captured_article_pushed_by_one_device_is_pulled_by_another() {
        let (base_url, _store) = spawn_mock_s3().await;
        let client = test_client(&base_url);
        let data_dir = tempfile::tempdir().unwrap();

        let device_a = migrated_conn();
        crate::db::queries::insert_captured_article(
            &device_a,
            "art-1",
            None,
            "direct",
            &sample_output("https://example.com/a", "Hello"),
            &["tag-a".to_string()],
        )
        .unwrap();

        let outcome_a = run_sync(
            &device_a,
            &client,
            data_dir.path(),
            Utc::now(),
            &no_cancel(),
            |_| {},
        )
        .await
        .expect("device A syncs cleanly against an empty bucket");
        assert_eq!(outcome_a.pushed, 1);

        let device_b = migrated_conn();
        let outcome_b = run_sync(
            &device_b,
            &client,
            data_dir.path(),
            Utc::now(),
            &no_cancel(),
            |_| {},
        )
        .await
        .expect("device B syncs cleanly");
        assert_eq!(outcome_b.pulled, 1);

        let pulled = sync_rows::get_article_for_sync(&device_b, "art-1")
            .unwrap()
            .expect("article must exist on device B after pulling");
        assert_eq!(pulled.title, "Hello");
        assert_eq!(pulled.content_html, "<p>Hello</p>");
        assert_eq!(pulled.tags, vec!["tag-a".to_string()]);
    }

    #[tokio::test]
    async fn a_delete_on_one_device_removes_the_article_on_another_after_both_sync() {
        let (base_url, _store) = spawn_mock_s3().await;
        let client = test_client(&base_url);
        let data_dir = tempfile::tempdir().unwrap();

        let device_a = migrated_conn();
        crate::db::queries::insert_captured_article(
            &device_a,
            "art-1",
            None,
            "direct",
            &sample_output("https://example.com/a", "Hello"),
            &[],
        )
        .unwrap();
        run_sync(
            &device_a,
            &client,
            data_dir.path(),
            Utc::now(),
            &no_cancel(),
            |_| {},
        )
        .await
        .unwrap();

        let device_b = migrated_conn();
        run_sync(
            &device_b,
            &client,
            data_dir.path(),
            Utc::now(),
            &no_cancel(),
            |_| {},
        )
        .await
        .unwrap();
        assert!(
            sync_rows::get_article_for_sync(&device_b, "art-1")
                .unwrap()
                .is_some()
        );

        // Device A deletes it and pushes the tombstone.
        crate::db::queries::delete_article(&device_a, "art-1").unwrap();
        run_sync(
            &device_a,
            &client,
            data_dir.path(),
            Utc::now(),
            &no_cancel(),
            |_| {},
        )
        .await
        .unwrap();

        // Device B syncs again and must remove its own copy.
        let outcome = run_sync(
            &device_b,
            &client,
            data_dir.path(),
            Utc::now(),
            &no_cancel(),
            |_| {},
        )
        .await
        .unwrap();
        assert_eq!(outcome.tombstones_applied, 1);
        assert!(
            sync_rows::get_article_for_sync(&device_b, "art-1")
                .unwrap()
                .is_none()
        );

        // And device B must never resurrect it even if it tries to push
        // a local copy again (it can't, since delete_article removed the
        // row — this asserts there is nothing left to push).
        let outcome2 = run_sync(
            &device_b,
            &client,
            data_dir.path(),
            Utc::now(),
            &no_cancel(),
            |_| {},
        )
        .await
        .unwrap();
        assert_eq!(outcome2.pushed, 0);
    }

    #[tokio::test]
    async fn two_devices_independently_capturing_the_same_link_converge_to_one_survivor() {
        let (base_url, _store) = spawn_mock_s3().await;
        let client = test_client(&base_url);
        let data_dir = tempfile::tempdir().unwrap();

        let device_a = migrated_conn();
        crate::db::queries::insert_captured_article(
            &device_a,
            "art-a",
            None,
            "direct",
            &sample_output("https://example.com/race", "From A"),
            &["from-a".to_string()],
        )
        .unwrap();
        // Force a deterministic, earlier fetched_at so A is the expected
        // winner regardless of real wall-clock timing in this test.
        device_a
            .execute(
                "UPDATE articles SET fetched_at = '2020-01-01T00:00:00Z', updated_at = '2020-01-01T00:00:00Z' WHERE id = 'art-a'",
                [],
            )
            .unwrap();

        let device_b = migrated_conn();
        crate::db::queries::insert_captured_article(
            &device_b,
            "art-b",
            None,
            "direct",
            &sample_output("https://example.com/race", "From B"),
            &["from-b".to_string()],
        )
        .unwrap();
        device_b
            .execute(
                "UPDATE articles SET fetched_at = '2021-01-01T00:00:00Z', updated_at = '2021-01-01T00:00:00Z', favorited = 1 WHERE id = 'art-b'",
                [],
            )
            .unwrap();

        // A pushes first, B pulls A's copy (creating the collision
        // locally), resolves it, and pushes the resolution.
        run_sync(
            &device_a,
            &client,
            data_dir.path(),
            Utc::now(),
            &no_cancel(),
            |_| {},
        )
        .await
        .unwrap();
        run_sync(
            &device_b,
            &client,
            data_dir.path(),
            Utc::now(),
            &no_cancel(),
            |_| {},
        )
        .await
        .unwrap();
        // A syncs again to pick up B's resolution.
        run_sync(
            &device_a,
            &client,
            data_dir.path(),
            Utc::now(),
            &no_cancel(),
            |_| {},
        )
        .await
        .unwrap();
        // B syncs once more so both fully converge on the same tombstone view.
        run_sync(
            &device_b,
            &client,
            data_dir.path(),
            Utc::now(),
            &no_cancel(),
            |_| {},
        )
        .await
        .unwrap();

        let a_rows = sync_rows::list_articles_for_sync(&device_a).unwrap();
        let b_rows = sync_rows::list_articles_for_sync(&device_b).unwrap();
        assert_eq!(
            a_rows.len(),
            1,
            "device A must converge to exactly one surviving row"
        );
        assert_eq!(
            b_rows.len(),
            1,
            "device B must converge to exactly one surviving row"
        );
        assert_eq!(
            a_rows[0].id, "art-a",
            "the earlier-created row must win deterministically"
        );
        assert_eq!(b_rows[0].id, "art-a");
        assert!(
            a_rows[0].tags.contains(&"from-b".to_string()),
            "the loser's tags must be merged into the survivor, not discarded"
        );
        assert!(
            a_rows[0].favorited,
            "the loser's favorited flag must be merged in"
        );
    }
}

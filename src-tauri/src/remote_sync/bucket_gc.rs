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
use super::engine::{SyncError, blob_key};
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

/// Deletes every blob under `legere-sync/blobs/` that isn't referenced by
/// `manifest`'s live entries and hasn't been modified within
/// `GC_GRACE_PERIOD`. Returns how many were deleted. A failure partway
/// through (one bad delete) stops the sweep but doesn't fail the sync
/// pass that triggered it — see this module's caller in `engine.rs`.
pub async fn sweep_orphaned_blobs(
    client: &S3Client,
    manifest: &Manifest,
    cancel: &Arc<AtomicBool>,
) -> Result<usize, SyncError> {
    let live_keys: HashSet<String> = manifest
        .live_entries()
        .map(|e| blob_key(e.entity_type, &e.id))
        .collect();

    let objects = client.list_objects_with_prefix(BLOBS_PREFIX).await?;
    let now = Utc::now();
    let mut deleted = 0;

    for object in objects {
        if cancel.load(Ordering::Relaxed) {
            return Err(SyncError::Cancelled);
        }
        if live_keys.contains(&object.key) {
            continue;
        }
        if !is_older_than_grace_period(&object.last_modified, now) {
            continue;
        }
        client.delete_object(&object.key).await?;
        deleted += 1;
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
}

//! The sync manifest: a small, versioned index of every article/category/
//! source row and tombstone known to exist across a user's synced devices.
//! It never carries article content itself (that lives in per-entity blobs
//! — see `client.rs` and ARCHITECTURE.md's Sync section) — only enough
//! per-row metadata for a device to diff it against its local DB and
//! decide what to pull, push, or delete locally. Kept small deliberately:
//! it's the one object every device contends on (see `client.rs`'s
//! conditional-write handling), so cheap to rewrite matters more here than
//! anywhere else in the sync design.

use std::collections::HashSet;
use std::io::{Read, Write};

use chrono::{DateTime, Duration, Utc};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};

/// The three entity types that sync. `settings` is deliberately absent —
/// it stays local per-device (see ARCHITECTURE.md's Sync section).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityType {
    Article,
    Category,
    Source,
}

impl EntityType {
    /// Matches `sync_tombstones.entity_type`'s CHECK constraint values
    /// (`db::schema`'s `V17`) exactly. `db::sync_rows::apply_tombstone`
    /// and friends take a plain `&str` rather than this enum, since
    /// they're shared with the local-delete tombstone-recording path,
    /// which has no reason to depend on this module.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Article => "article",
            Self::Category => "category",
            Self::Source => "source",
        }
    }
}

/// One row's sync-relevant fingerprint. `conflict_key` backs the
/// deterministic-winner merge for same-identity races (two devices
/// independently capturing the same article link, adding the same
/// category name, or adding the same RSS feed before either has synced)
/// — see `conflict::pick_winner`. `conflict_key` is always the entity's
/// natural key (article `link`, category `lower(name)`, source
/// `feed_url`), never the row `id`, since the race is exactly two
/// different ids claiming the same real-world identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub id: String,
    pub entity_type: EntityType,
    pub conflict_key: String,
    /// RFC 3339, matches every synced table's `updated_at` column —
    /// drives plain last-write-wins for non-colliding updates.
    pub updated_at: String,
    /// RFC 3339 — the entity's own `created_at`/`fetched_at`, used only
    /// to break a `conflict_key` collision deterministically (earlier
    /// creation wins). Distinct from `updated_at`, which keeps moving
    /// every time the row's mutable fields change.
    pub created_at: String,
    /// Hash of the entity's blob(s) (metadata + content, and for
    /// articles the hero thumbnail) — lets a device skip pulling a blob
    /// it already has the current version of.
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TombstoneEntry {
    pub entity_type: EntityType,
    pub entity_id: String,
    pub deleted_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Manifest {
    /// Bumped on every successful write. Purely informational — carried
    /// so a human inspecting the bucket can see sync activity — the
    /// actual concurrency guard is the object store's ETag via
    /// conditional writes (see `client.rs`), not this counter.
    pub version: u64,
    pub entries: Vec<ManifestEntry>,
    pub tombstones: Vec<TombstoneEntry>,
}

#[derive(Debug)]
pub enum ManifestError {
    Gzip(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gzip(e) => write!(f, "failed to decompress manifest: {e}"),
            Self::Json(e) => write!(f, "failed to parse manifest JSON: {e}"),
        }
    }
}

impl std::error::Error for ManifestError {}

impl Manifest {
    /// Encodes as gzip-compressed JSON. Manifests are diffed and rewritten
    /// on every sync, so this favors decode speed over Manifest-specific
    /// compactness — `flate2`'s default pure-Rust backend, matching
    /// `db::compression`'s existing choice (no new C dependency, no
    /// Android NDK cross-compilation risk).
    pub fn to_gz_bytes(&self) -> Vec<u8> {
        let json = serde_json::to_vec(self).expect("Manifest serialization cannot fail");
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        encoder
            .write_all(&json)
            .expect("writing to an in-memory Vec<u8> should never fail");
        encoder
            .finish()
            .expect("finishing an in-memory gzip stream should never fail")
    }

    pub fn from_gz_bytes(bytes: &[u8]) -> Result<Self, ManifestError> {
        let mut decoder = GzDecoder::new(bytes);
        let mut json = Vec::new();
        decoder.read_to_end(&mut json).map_err(ManifestError::Gzip)?;
        serde_json::from_slice(&json).map_err(ManifestError::Json)
    }

    /// Entries with no matching tombstone. A tombstoned id must never be
    /// materialized locally regardless of what its (possibly stale)
    /// `entries` row still says — see ARCHITECTURE.md's Sync section's
    /// "tombstone gates everything" rule. This can't be enforced once at
    /// load time and forgotten: a tombstone can arrive in the very same
    /// manifest version as a stale entry left over from a device that
    /// raced the delete, so every read of `entries` goes through this.
    pub fn live_entries(&self) -> impl Iterator<Item = &ManifestEntry> {
        let tombstoned: HashSet<&str> = self
            .tombstones
            .iter()
            .map(|t| t.entity_id.as_str())
            .collect();
        self.entries
            .iter()
            .filter(move |e| !tombstoned.contains(e.id.as_str()))
    }

    /// Drops tombstones older than `retention`. Called before every
    /// manifest write rather than on a separate schedule, so the purge is
    /// naturally rate-limited to "at most once per sync" and needs no
    /// extra coordination between devices. Anything whose `deleted_at`
    /// fails to parse is kept rather than risk purging it incorrectly.
    pub fn purge_expired_tombstones(&mut self, now: DateTime<Utc>, retention: Duration) {
        self.tombstones.retain(|t| {
            DateTime::parse_from_rfc3339(&t.deleted_at)
                .map(|dt| now.signed_duration_since(dt) < retention)
                .unwrap_or(true)
        });
    }
}

/// The tombstone retention window (see schema `V17`'s doc comment): fixed,
/// not user-configurable. A device offline longer than this risks
/// resurrecting an old delete on reconnect — an accepted, low-severity
/// trade against the complexity of a device-ack quorum for a tool aimed
/// at 2-3 personal devices.
pub const TOMBSTONE_RETENTION: Duration = Duration::days(60);

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(id: &str) -> ManifestEntry {
        ManifestEntry {
            id: id.to_string(),
            entity_type: EntityType::Article,
            conflict_key: "https://example.com/a".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            content_hash: "deadbeef".to_string(),
        }
    }

    #[test]
    fn round_trips_through_gzip_json() {
        let manifest = Manifest {
            version: 3,
            entries: vec![sample_entry("art-1")],
            tombstones: vec![TombstoneEntry {
                entity_type: EntityType::Source,
                entity_id: "src-1".to_string(),
                deleted_at: "2026-01-02T00:00:00Z".to_string(),
            }],
        };

        let bytes = manifest.to_gz_bytes();
        let decoded = Manifest::from_gz_bytes(&bytes).expect("valid gzip+json round-trips");
        assert_eq!(decoded, manifest);
    }

    #[test]
    fn live_entries_excludes_anything_tombstoned() {
        let manifest = Manifest {
            version: 1,
            entries: vec![sample_entry("art-1"), sample_entry("art-2")],
            tombstones: vec![TombstoneEntry {
                entity_type: EntityType::Article,
                entity_id: "art-1".to_string(),
                deleted_at: "2026-01-01T00:00:00Z".to_string(),
            }],
        };

        let ids: Vec<&str> = manifest.live_entries().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["art-2"]);
    }

    #[test]
    fn purge_expired_tombstones_drops_only_what_is_past_retention() {
        let mut manifest = Manifest {
            version: 1,
            entries: vec![],
            tombstones: vec![
                TombstoneEntry {
                    entity_type: EntityType::Article,
                    entity_id: "old".to_string(),
                    deleted_at: "2025-01-01T00:00:00Z".to_string(),
                },
                TombstoneEntry {
                    entity_type: EntityType::Article,
                    entity_id: "recent".to_string(),
                    deleted_at: "2026-06-01T00:00:00Z".to_string(),
                },
            ],
        };

        let now = DateTime::parse_from_rfc3339("2026-06-15T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        manifest.purge_expired_tombstones(now, TOMBSTONE_RETENTION);

        let ids: Vec<&str> = manifest
            .tombstones
            .iter()
            .map(|t| t.entity_id.as_str())
            .collect();
        assert_eq!(ids, vec!["recent"]);
    }

    #[test]
    fn purge_keeps_a_tombstone_whose_timestamp_fails_to_parse() {
        let mut manifest = Manifest {
            version: 1,
            entries: vec![],
            tombstones: vec![TombstoneEntry {
                entity_type: EntityType::Article,
                entity_id: "malformed".to_string(),
                deleted_at: "not-a-timestamp".to_string(),
            }],
        };

        manifest.purge_expired_tombstones(Utc::now(), TOMBSTONE_RETENTION);
        assert_eq!(manifest.tombstones.len(), 1, "unparseable timestamps must not be purged");
    }
}

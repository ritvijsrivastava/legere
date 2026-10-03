//! Cross-device sync's own local configuration: the user's bucket
//! credentials and this device's identity. Stored in the same `settings`
//! KV table the app's UI preferences already use (under a `remote_sync_`
//! key prefix `queries::get_settings`'s unknown-key fallthrough ignores),
//! not a new table — the property this data needs, "never synced, always
//! local to this device," is exactly what `settings` already guarantees
//! for everything else it holds. See ARCHITECTURE.md's Sync section.

use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::remote_sync::credential_vault;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteSyncConfig {
    pub enabled: bool,
    pub endpoint: String,
    pub bucket_name: String,
    pub region: String,
    /// `true` for most self-hosted/non-AWS providers (Minio, some B2
    /// setups); `false` for R2/AWS, matching each provider's documented
    /// URL convention — see `remote_sync::client::BucketConfig`.
    pub use_path_style: bool,
    pub access_key: String,
    pub secret_key: String,
    /// Whether `access_key`/`secret_key` are currently encrypted at rest
    /// (`remote_sync::credential_vault`) rather than stored as plaintext
    /// -- read-only, like `device_id` below: whatever the caller's
    /// `config.credentials_encrypted` says on a save is ignored, since
    /// this reflects what actually happened on *this* save (did a
    /// platform key store turn out to be reachable or not), not
    /// something a caller gets to assert. Drives the Settings screen's
    /// "stored unencrypted" warning and its manual retry action
    /// ([`retry_credential_encryption`]).
    pub credentials_encrypted: bool,
    /// Generated once on first save and never changed — identifies this
    /// device's own writes for diagnostics (`devices/<device_id>.json` in
    /// the bucket layout), not used in any conflict-resolution decision.
    pub device_id: String,
    /// Purely informational, reflecting whatever the Settings screen's
    /// "Test connection" button (`commands::remote_sync::test_remote_sync_connection`,
    /// which runs `S3Client::probe_conditional_write_support`) last found
    /// for the bucket fields it was tested against — not re-verified by
    /// `save_remote_sync_config` on every save (that used to run the same
    /// four-request probe on *every* save regardless of relevance, making
    /// e.g. a plain `sync_interval_hours` change visibly take several
    /// seconds). Trusted as given from the frontend here, unlike
    /// `device_id` below — `RemoteSyncDialog`'s `draftConfig` is what
    /// keeps it honest, resetting to `false` the moment any
    /// bucket-identifying field is edited after a successful test so a
    /// stale `true` can't survive unverified. See
    /// `S3Client::probe_conditional_write_support`'s doc comment for why
    /// an unsupported provider is rejected outright rather than degraded
    /// to a weaker fallback — this field just records the result, it
    /// isn't itself enforced before a sync pass is allowed to run.
    pub conditional_writes_verified: bool,
    /// How often the background sync loop runs, in hours. Clamped to
    /// 1-24 wherever it's read or written (`get_remote_sync_config`/
    /// `save_remote_sync_config`) rather than trusted as free-form input
    /// -- unlike RSS autosync's fixed 6/12/24 choices, a free range is
    /// fine here since there's no shared-resource cost to a finer value,
    /// just the user's own bucket request volume.
    pub sync_interval_hours: i64,
}

/// Default/fallback for `sync_interval_hours` when unset -- matches
/// `RemoteSyncConfig`'s implicit default before this field existed.
const DEFAULT_SYNC_INTERVAL_HOURS: i64 = 6;

fn clamp_sync_interval_hours(hours: i64) -> i64 {
    hours.clamp(1, 24)
}

fn get_setting(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .optional()
}

fn set_setting(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// `None` when sync has never been configured on this device (no
/// `remote_sync_bucket_name` saved yet) — the caller's cue to show setup
/// rather than a sync status/error.
fn ensure_device_id(conn: &Connection) -> rusqlite::Result<String> {
    match get_setting(conn, "remote_sync_device_id")? {
        Some(id) => Ok(id),
        None => {
            let id = Uuid::new_v4().to_string();
            set_setting(conn, "remote_sync_device_id", &id)?;
            Ok(id)
        }
    }
}

pub fn get_remote_sync_config(conn: &Connection) -> rusqlite::Result<Option<RemoteSyncConfig>> {
    let Some(bucket_name) = get_setting(conn, "remote_sync_bucket_name")? else {
        return Ok(None);
    };
    let device_id = ensure_device_id(conn)?;

    let stored_access_key = get_setting(conn, "remote_sync_access_key")?.unwrap_or_default();
    let stored_secret_key = get_setting(conn, "remote_sync_secret_key")?.unwrap_or_default();
    let (access_key, secret_key, credentials_encrypted) =
        decrypt_stored_credentials(&stored_access_key, &stored_secret_key);

    Ok(Some(RemoteSyncConfig {
        enabled: get_setting(conn, "remote_sync_enabled")?.as_deref() == Some("true"),
        endpoint: get_setting(conn, "remote_sync_endpoint")?.unwrap_or_default(),
        bucket_name,
        region: get_setting(conn, "remote_sync_region")?.unwrap_or_default(),
        use_path_style: get_setting(conn, "remote_sync_use_path_style")?.as_deref() == Some("true"),
        access_key,
        secret_key,
        credentials_encrypted,
        device_id,
        conditional_writes_verified: get_setting(conn, "remote_sync_conditional_writes_verified")?
            .as_deref()
            == Some("true"),
        sync_interval_hours: get_setting(conn, "remote_sync_interval_hours")?
            .and_then(|value| value.parse::<i64>().ok())
            .map(clamp_sync_interval_hours)
            .unwrap_or(DEFAULT_SYNC_INTERVAL_HOURS),
    }))
}

/// Decrypts `stored_access_key`/`stored_secret_key` as read from the
/// `settings` table, returning the plaintext pair plus whether both were
/// actually encrypted. A decrypt failure on an encrypted value (key gone,
/// corrupted data — see `credential_vault`'s doc comment) degrades to an
/// empty string for that field rather than failing the whole config load:
/// the rest of `RemoteSyncConfig` (interval, enabled, device id, ...) is
/// still perfectly usable, and an empty credential just means the
/// Settings screen prompts the user to re-enter it, the same recoverable
/// state as a never-configured device.
fn decrypt_stored_credentials(access_key: &str, secret_key: &str) -> (String, String, bool) {
    let both_tagged =
        credential_vault::is_encrypted(access_key) && credential_vault::is_encrypted(secret_key);

    let decrypt_one = |value: &str| match credential_vault::decrypt(value) {
        Ok(plain) => plain,
        Err(error) => {
            tracing::warn!(%error, "failed to decrypt stored remote sync credential");
            String::new()
        }
    };

    (
        decrypt_one(access_key),
        decrypt_one(secret_key),
        both_tagged,
    )
}

/// Saves every field except `device_id`, which `ensure_device_id` owns
/// entirely: whatever the caller's `config.device_id` says is ignored,
/// not merely overridden on first save, since trusting client input for
/// this field is exactly the bug this function used to have (a
/// not-yet-configured frontend has no real id to send yet).
pub fn save_remote_sync_config(
    conn: &Connection,
    config: &RemoteSyncConfig,
) -> rusqlite::Result<()> {
    ensure_device_id(conn)?;

    set_setting(
        conn,
        "remote_sync_enabled",
        if config.enabled { "true" } else { "false" },
    )?;
    set_setting(conn, "remote_sync_endpoint", &config.endpoint)?;
    set_setting(conn, "remote_sync_bucket_name", &config.bucket_name)?;
    set_setting(conn, "remote_sync_region", &config.region)?;
    set_setting(
        conn,
        "remote_sync_use_path_style",
        if config.use_path_style {
            "true"
        } else {
            "false"
        },
    )?;
    let (access_key, secret_key, _) =
        encrypt_credentials_for_storage(&config.access_key, &config.secret_key);
    set_setting(conn, "remote_sync_access_key", &access_key)?;
    set_setting(conn, "remote_sync_secret_key", &secret_key)?;
    set_setting(
        conn,
        "remote_sync_conditional_writes_verified",
        if config.conditional_writes_verified {
            "true"
        } else {
            "false"
        },
    )?;
    set_setting(
        conn,
        "remote_sync_interval_hours",
        &clamp_sync_interval_hours(config.sync_interval_hours).to_string(),
    )?;
    Ok(())
}

/// Encrypts `access_key`/`secret_key` for storage if a platform key store
/// is reachable right now; otherwise returns them unchanged (today's
/// plaintext behavior) -- see `credential_vault`'s doc comment for why
/// this never fails outright. Both fields share one "was a key store
/// available" outcome: there's only ever one key per device, so either
/// both get encrypted or neither does, never a mix.
fn encrypt_credentials_for_storage(access_key: &str, secret_key: &str) -> (String, String, bool) {
    match (
        credential_vault::encrypt(access_key),
        credential_vault::encrypt(secret_key),
    ) {
        (Some(access_key), Some(secret_key)) => (access_key, secret_key, true),
        _ => (access_key.to_string(), secret_key.to_string(), false),
    }
}

/// Re-attempts encrypting the currently-stored credentials in place --
/// the Settings screen's manual "Retry" action, for when a platform key
/// store wasn't reachable at the time of the last save (e.g. no Secret
/// Service was running yet) but might be now. A no-op, successfully,
/// if credentials are already encrypted or there's nothing configured
/// yet. Returns whether credentials are encrypted after this call.
pub fn retry_credential_encryption(conn: &Connection) -> rusqlite::Result<bool> {
    let Some(config) = get_remote_sync_config(conn)? else {
        return Ok(false);
    };
    if config.credentials_encrypted {
        return Ok(true);
    }
    let (access_key, secret_key, encrypted) =
        encrypt_credentials_for_storage(&config.access_key, &config.secret_key);
    set_setting(conn, "remote_sync_access_key", &access_key)?;
    set_setting(conn, "remote_sync_secret_key", &secret_key)?;
    Ok(encrypted)
}

/// Status surfaced by the "Sync now" button / settings screen — separate
/// from `RemoteSyncConfig` since these change on every sync run, not just
/// when the user edits their bucket settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RemoteSyncStatus {
    pub last_synced_at: Option<String>,
    pub last_error: Option<String>,
}

pub fn get_remote_sync_status(conn: &Connection) -> rusqlite::Result<RemoteSyncStatus> {
    Ok(RemoteSyncStatus {
        last_synced_at: get_setting(conn, "remote_sync_last_synced_at")?,
        last_error: get_setting(conn, "remote_sync_last_error")?,
    })
}

pub fn record_sync_success(conn: &Connection) -> rusqlite::Result<()> {
    set_setting(conn, "remote_sync_last_synced_at", &Utc::now().to_rfc3339())?;
    conn.execute(
        "DELETE FROM settings WHERE key = 'remote_sync_last_error'",
        [],
    )?;
    Ok(())
}

pub fn record_sync_failure(conn: &Connection, error: &str) -> rusqlite::Result<()> {
    set_setting(conn, "remote_sync_last_error", error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migrated_conn() -> Connection {
        let mut conn = Connection::open_in_memory().expect("open in-memory db");
        crate::db::schema::migrate(&mut conn).expect("migrate");
        conn
    }

    fn sample_config() -> RemoteSyncConfig {
        RemoteSyncConfig {
            enabled: true,
            endpoint: "https://example.r2.cloudflarestorage.com".to_string(),
            bucket_name: "legere-sync".to_string(),
            region: "auto".to_string(),
            use_path_style: false,
            access_key: "key".to_string(),
            secret_key: "secret".to_string(),
            credentials_encrypted: false,
            device_id: "will-be-ignored".to_string(),
            conditional_writes_verified: true,
            sync_interval_hours: 6,
        }
    }

    #[test]
    fn returns_none_before_anything_is_configured() {
        let conn = migrated_conn();
        assert_eq!(get_remote_sync_config(&conn).unwrap(), None);
    }

    #[test]
    fn saves_and_round_trips_a_config() {
        let conn = migrated_conn();
        save_remote_sync_config(&conn, &sample_config()).unwrap();

        let loaded = get_remote_sync_config(&conn).unwrap().unwrap();
        assert_eq!(loaded.bucket_name, "legere-sync");
        assert!(loaded.enabled);
        assert!(loaded.conditional_writes_verified);
        assert!(!loaded.device_id.is_empty());
        assert_eq!(loaded.sync_interval_hours, 6);
    }

    #[test]
    fn get_remote_sync_config_defaults_sync_interval_hours_to_six_when_unset() {
        let conn = migrated_conn();
        // `save_remote_sync_config` always writes this key, so simulate a
        // pre-existing device by saving everything else first, then
        // deleting just the interval row.
        save_remote_sync_config(&conn, &sample_config()).unwrap();
        conn.execute(
            "DELETE FROM settings WHERE key = 'remote_sync_interval_hours'",
            [],
        )
        .unwrap();
        assert_eq!(
            get_remote_sync_config(&conn)
                .unwrap()
                .unwrap()
                .sync_interval_hours,
            6
        );
    }

    #[test]
    fn save_remote_sync_config_clamps_sync_interval_hours_to_one_and_twenty_four() {
        let conn = migrated_conn();
        let mut config = sample_config();
        config.sync_interval_hours = 0;
        save_remote_sync_config(&conn, &config).unwrap();
        assert_eq!(
            get_remote_sync_config(&conn)
                .unwrap()
                .unwrap()
                .sync_interval_hours,
            1
        );

        config.sync_interval_hours = 48;
        save_remote_sync_config(&conn, &config).unwrap();
        assert_eq!(
            get_remote_sync_config(&conn)
                .unwrap()
                .unwrap()
                .sync_interval_hours,
            24
        );
    }

    #[test]
    fn device_id_is_generated_once_and_stable_across_saves() {
        let conn = migrated_conn();
        save_remote_sync_config(&conn, &sample_config()).unwrap();
        let first_id = get_remote_sync_config(&conn).unwrap().unwrap().device_id;

        let mut second_config = sample_config();
        second_config.device_id = "attempted-override".to_string();
        save_remote_sync_config(&conn, &second_config).unwrap();

        let second_id = get_remote_sync_config(&conn).unwrap().unwrap().device_id;
        assert_eq!(first_id, second_id, "device_id must never change once set");
    }

    #[test]
    fn first_ever_save_never_persists_the_frontend_supplied_device_id() {
        // Regression test: before any device_id exists yet (a brand-new
        // device that has never called get_remote_sync_config with a
        // bucket already configured), the very first save must not take
        // config.device_id at face value - a fresh frontend form has
        // nothing real to put there.
        let conn = migrated_conn();
        let mut config = sample_config();
        config.device_id = String::new();

        save_remote_sync_config(&conn, &config).unwrap();

        let saved_id = get_remote_sync_config(&conn).unwrap().unwrap().device_id;
        assert!(
            !saved_id.is_empty(),
            "an empty client-supplied device_id must never be persisted"
        );
        assert_eq!(
            saved_id.len(),
            36,
            "a real UUID, not whatever the client happened to send"
        );
    }

    #[test]
    fn record_sync_success_clears_a_previous_error() {
        let conn = migrated_conn();
        save_remote_sync_config(&conn, &sample_config()).unwrap();
        record_sync_failure(&conn, "boom").unwrap();
        assert_eq!(
            get_remote_sync_status(&conn).unwrap().last_error,
            Some("boom".to_string())
        );

        record_sync_success(&conn).unwrap();
        let status = get_remote_sync_status(&conn).unwrap();
        assert_eq!(status.last_error, None);
        assert!(status.last_synced_at.is_some());
    }
}

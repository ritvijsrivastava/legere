//! Linux `KeySource`: a random 32-byte key stored in the desktop Secret
//! Service (gnome-keyring, kwallet's `ksecrets` module, ...) via the
//! `keyring` crate's `v1` API, which auto-selects the right backend for
//! the running platform. Unlike Android's Keystore, Secret Service stores
//! arbitrary secret bytes directly — no separate "key encrypts a key"
//! indirection needed, this *is* the key [`super::encrypt`]/
//! [`super::decrypt`] use.
//!
//! Whether this is actually reachable depends on the desktop session, not
//! just the OS — see `ARCHITECTURE.md`'s Sync section for the full
//! breakdown (full GNOME/KDE session: unlocked transparently via PAM at
//! login; bare window manager: may prompt once, or have no Secret Service
//! provider running at all). Every failure mode collapses to
//! [`super::VaultError::KeyUnavailable`] here — `db::sync_config` is the
//! layer that turns that into "store the plaintext instead."

use keyring::Entry;
use rand::RngCore;
use rand::rngs::OsRng;

use super::{KeySource, VaultError};

/// Debug builds (dev runs, `cargo test`) use their own Secret Service
/// entry so they never read or create the installed app's key.
const SERVICE: &str = if cfg!(debug_assertions) {
    "legere-dev"
} else {
    "legere"
};
const ACCOUNT: &str = "remote-sync-credential-key";

pub struct LinuxKeySource;

impl KeySource for LinuxKeySource {
    fn get_or_create_key() -> Result<[u8; 32], VaultError> {
        let entry = Entry::new(SERVICE, ACCOUNT).map_err(|_| VaultError::KeyUnavailable)?;

        match entry.get_secret() {
            Ok(bytes) => to_key_array(&bytes).ok_or(VaultError::KeyUnavailable),
            Err(keyring::Error::NoEntry) => {
                // First use on this device: generate once, store, and
                // hand back the same key this call will keep returning
                // from here on.
                let mut key = [0u8; 32];
                OsRng.fill_bytes(&mut key);
                entry
                    .set_secret(&key)
                    .map_err(|_| VaultError::KeyUnavailable)?;
                Ok(key)
            }
            // Anything else (no Secret Service running, locked and the
            // user declined the unlock prompt, D-Bus unreachable, ...) is
            // "unavailable right now" -- deliberately not retried with a
            // fresh `set_secret` here, unlike the `NoEntry` case above:
            // if the entry genuinely exists but can't be reached, writing
            // a new one over it would risk orphaning whatever was already
            // encrypted with the original key.
            Err(_) => Err(VaultError::KeyUnavailable),
        }
    }
}

/// `None` if `bytes` isn't exactly a 32-byte key -- defends against a
/// corrupted or manually-tampered keyring entry without silently
/// overwriting it (see the doc comment above for why overwriting is
/// avoided even in the `NoEntry` case's sibling failure modes).
fn to_key_array(bytes: &[u8]) -> Option<[u8; 32]> {
    let key: [u8; 32] = bytes.try_into().ok()?;
    Some(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exercises the real Secret Service, not a fake -- so this only
    /// asserts the shape of the outcome, not that a key store is
    /// actually reachable in whatever environment runs this test (a CI
    /// runner or a bare window manager may have no Secret Service
    /// provider at all, same as a real user's machine might not). A
    /// single test (rather than two separate `#[test]` fns) deliberately
    /// -- both calls share the same fixed `(SERVICE, ACCOUNT)` identity,
    /// and `cargo test`'s default thread-parallel test execution would
    /// otherwise race two independently-running tests' generate-and-store
    /// against that one shared entry (confirmed empirically: splitting
    /// this into two `#[test]` fns intermittently produced two different
    /// "first-ever" keys, one per racing test, each overwriting the
    /// other's `set_secret`).
    #[test]
    fn get_or_create_key_never_panics_and_is_stable_when_available() {
        let first = match LinuxKeySource::get_or_create_key() {
            Ok(key) => key,
            Err(VaultError::KeyUnavailable) => return, // no Secret Service reachable here
            Err(other) => panic!("unexpected error variant: {other:?}"),
        };
        assert_eq!(first.len(), 32);

        // When a key store is reachable, a second call in the same
        // process must return the identical key -- otherwise every
        // previously-encrypted credential becomes silently undecryptable
        // the next time the app starts.
        let second = LinuxKeySource::get_or_create_key().expect("key store was just reachable");
        assert_eq!(first, second);
    }
}

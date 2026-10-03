//! At-rest encryption for the bucket `access_key`/`secret_key`
//! `db::sync_config` stores, keyed by a per-platform secret that never
//! itself touches `legere.db` — so a copy of the database file alone
//! (the thing commodity infostealer malware actually goes after: bulk
//! scraping known SQLite paths for credentials) isn't enough to recover
//! them. See ARCHITECTURE.md's Sync section for the full rationale and
//! the threat model this does and doesn't cover.
//!
//! Platform split:
//! - Linux: [`linux::LinuxKeySource`] stores a random 32-byte key in the
//!   desktop Secret Service (gnome-keyring/kwallet) via the `keyring`
//!   crate. That key then encrypts the credentials here, the same way
//!   every platform below does.
//! - Android: [`android::AndroidKeySource`] skips the "store a random key
//!   in the OS store" step entirely — the Android Keystore's own AES key
//!   is already non-exportable and hardware-backed, so it encrypts the
//!   credentials directly. See that module's doc comment for why this
//!   crate's own (third-party) Android backend isn't used instead.
//! - Everything else (desktop dev builds on an unsupported OS, CI): no
//!   key source at all — [`encrypt`] returns `None` unconditionally,
//!   which callers already have to handle as "store the plaintext".
//!
//! Not a general-purpose secrets API: this only ever encrypts/decrypts
//! one string at a time, with one fixed (service, account) identity per
//! platform, exactly the shape `db::sync_config` needs and nothing more.
//!
//! [`encrypt`] never fails — "no platform key store available right now"
//! degrades to [`None`], which the caller treats as "store this value as
//! plain text", exactly today's (pre-encryption) behavior. [`decrypt`] on
//! an *unencrypted* value is the identity function for the same reason:
//! every row saved before this module existed, and every row saved on a
//! device/session where the key store was unavailable, is a bare string
//! forever until the next successful save re-encrypts it. [`decrypt`] can
//! fail, though: once a value genuinely is a tagged envelope, a decrypt
//! failure (key gone, corrupted data) is a real error, not something to
//! silently paper over by handing back garbage credentials.

use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use rand::RngCore;
use rand::rngs::OsRng;
use thiserror::Error;

#[cfg(target_os = "android")]
pub(crate) mod android;
#[cfg(target_os = "linux")]
mod linux;

/// Prefixes every envelope this module produces, so [`decrypt`] (and
/// [`is_encrypted`]) can tell a tagged ciphertext apart from the bare
/// plaintext strings every row saved before this module existed (or
/// saved with no key store available) already contains. Versioned on the
/// off chance the envelope shape ever needs to change.
const ENVELOPE_PREFIX: &str = "legere-enc-v1:";

/// AES-GCM's standard nonce size — matches `remote_sync::link`'s own
/// envelope, which this mirrors in spirit (random nonce per encryption,
/// prepended to the ciphertext) without that module's Argon2id step: the
/// key here already comes directly from a platform key store at full
/// entropy, not derived from a short human-typed passphrase.
const NONCE_LEN: usize = 12;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum VaultError {
    /// No platform key store is reachable right now (not installed, not
    /// running, user declined an unlock prompt, ...). Not necessarily
    /// permanent — a later retry (the Settings screen's manual "Retry"
    /// action) may succeed once the underlying cause is fixed.
    #[error("no platform key store available")]
    KeyUnavailable,
    /// A tagged envelope failed to decrypt: wrong/rotated key, corrupted
    /// data, or truncated input. Deliberately one variant covering all
    /// three — like `remote_sync::link`'s `DecryptFailed`, AES-GCM's
    /// authentication tag makes them indistinguishable by design, and
    /// there's no safe way to tell a caller which one happened.
    #[error("failed to decrypt stored credential")]
    DecryptFailed,
}

/// A per-platform source of the raw 256-bit key [`encrypt`]/[`decrypt`]
/// use. `get_or_create_key` creates the key on first call if the
/// platform's store doesn't have one yet; every later call returns the
/// same key. Implementations must never block on UI the caller didn't
/// expect — see each platform module's own doc comment for what "no key
/// store available" actually covers on that platform.
trait KeySource {
    fn get_or_create_key() -> Result<[u8; 32], VaultError>;
}

#[cfg(target_os = "android")]
use android::AndroidKeySource as PlatformKeySource;
#[cfg(target_os = "linux")]
use linux::LinuxKeySource as PlatformKeySource;

#[cfg(not(any(target_os = "linux", target_os = "android")))]
struct PlatformKeySource;
#[cfg(not(any(target_os = "linux", target_os = "android")))]
impl KeySource for PlatformKeySource {
    fn get_or_create_key() -> Result<[u8; 32], VaultError> {
        Err(VaultError::KeyUnavailable)
    }
}

/// True if `stored` is a tagged envelope this module produced — lets
/// `db::sync_config` report whether credentials are currently encrypted
/// without actually decrypting them.
pub fn is_encrypted(stored: &str) -> bool {
    stored.starts_with(ENVELOPE_PREFIX)
}

/// Encrypts `plaintext` for storage. `None` means "no platform key store
/// available right now" — the caller's cue to store `plaintext` as-is,
/// exactly like every row saved before this module existed.
pub fn encrypt(plaintext: &str) -> Option<String> {
    let key = PlatformKeySource::get_or_create_key().ok()?;
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));
    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes()).ok()?;

    let mut combined = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);
    Some(format!("{ENVELOPE_PREFIX}{}", BASE64.encode(combined)))
}

/// Decrypts `stored`. Returns it unchanged, successfully, if it isn't a
/// tagged envelope at all (see this module's doc comment for why that's
/// the identity function rather than an error). Only a *tagged* envelope
/// that can't be decrypted is a [`VaultError::DecryptFailed`].
pub fn decrypt(stored: &str) -> Result<String, VaultError> {
    let Some(encoded) = stored.strip_prefix(ENVELOPE_PREFIX) else {
        return Ok(stored.to_string());
    };

    let key = PlatformKeySource::get_or_create_key().map_err(|_| VaultError::DecryptFailed)?;
    let combined = BASE64
        .decode(encoded)
        .map_err(|_| VaultError::DecryptFailed)?;
    if combined.len() < NONCE_LEN {
        return Err(VaultError::DecryptFailed);
    }
    let (nonce_bytes, ciphertext) = combined.split_at(NONCE_LEN);

    let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| VaultError::DecryptFailed)?;
    String::from_utf8(plaintext).map_err(|_| VaultError::DecryptFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixed, in-memory-only key source, independent of whichever real
    /// platform store (or lack of one) this is compiled/run against — so
    /// the envelope format itself is tested the same way on every CI
    /// platform, without needing a real Secret Service/Keystore.
    struct FixedKeySource;
    impl KeySource for FixedKeySource {
        fn get_or_create_key() -> Result<[u8; 32], VaultError> {
            Ok([0x42; 32])
        }
    }

    fn encrypt_with_fixed_key(plaintext: &str) -> String {
        let key = FixedKeySource::get_or_create_key().unwrap();
        let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));
        let mut nonce_bytes = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes()).unwrap();
        let mut combined = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);
        format!("{ENVELOPE_PREFIX}{}", BASE64.encode(combined))
    }

    fn decrypt_with_fixed_key(stored: &str) -> Result<String, VaultError> {
        let Some(encoded) = stored.strip_prefix(ENVELOPE_PREFIX) else {
            return Ok(stored.to_string());
        };
        let key = FixedKeySource::get_or_create_key().map_err(|_| VaultError::DecryptFailed)?;
        let combined = BASE64
            .decode(encoded)
            .map_err(|_| VaultError::DecryptFailed)?;
        if combined.len() < NONCE_LEN {
            return Err(VaultError::DecryptFailed);
        }
        let (nonce_bytes, ciphertext) = combined.split_at(NONCE_LEN);
        let cipher = Aes256Gcm::new(GenericArray::from_slice(&key));
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| VaultError::DecryptFailed)?;
        String::from_utf8(plaintext).map_err(|_| VaultError::DecryptFailed)
    }

    #[test]
    fn round_trips_through_the_envelope_format() {
        let envelope = encrypt_with_fixed_key("super-secret-key");
        assert!(is_encrypted(&envelope));
        assert_eq!(
            decrypt_with_fixed_key(&envelope).unwrap(),
            "super-secret-key"
        );
    }

    #[test]
    fn decrypting_a_bare_plaintext_value_is_the_identity_function() {
        assert_eq!(
            decrypt_with_fixed_key("plain-old-access-key").unwrap(),
            "plain-old-access-key"
        );
        assert!(!is_encrypted("plain-old-access-key"));
    }

    #[test]
    fn tampered_ciphertext_fails_to_decrypt_rather_than_returning_garbage() {
        let mut envelope = encrypt_with_fixed_key("super-secret-key");
        envelope.push('x');
        assert_eq!(
            decrypt_with_fixed_key(&envelope),
            Err(VaultError::DecryptFailed)
        );
    }

    // `encrypt`/`decrypt` themselves (as opposed to the envelope format
    // above) aren't exercised here with a real `PlatformKeySource`:
    // that's inherently platform- and environment-dependent (a Linux dev
    // machine may or may not have a reachable Secret Service), which is
    // exactly the condition `linux`'s own tests are written to tolerate
    // explicitly rather than assert a single outcome here.
}

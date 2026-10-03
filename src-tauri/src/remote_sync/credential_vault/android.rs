//! Android `KeySource` -- placeholder. Always reports
//! [`VaultError::KeyUnavailable`], so `db::sync_config` falls back to
//! storing credentials as plaintext on Android exactly like it did before
//! this module existed, until the real Android Keystore-backed bridge
//! (a small Kotlin `MainActivity` method pair + the same cached-Activity
//! JNI pattern `import_intent.rs` already uses, called through here)
//! replaces this.

use super::{KeySource, VaultError};

pub struct AndroidKeySource;

impl KeySource for AndroidKeySource {
    fn get_or_create_key() -> Result<[u8; 32], VaultError> {
        Err(VaultError::KeyUnavailable)
    }
}

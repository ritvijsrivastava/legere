//! Android `KeySource`: unlike Linux's Secret Service (which stores
//! arbitrary secret bytes directly), the Android Keystore's primitive is
//! a non-exportable AES key usable only through `Cipher` calls inside the
//! JVM -- there's no way to hand its raw bytes back to Rust by design.
//! So this doesn't skip the DEK indirection the way an earlier design
//! draft assumed it could: a random 32-byte DEK is still generated and
//! owned by Rust, exactly like Linux, except here it's *wrapped*
//! (encrypted) by the Keystore key via a small Kotlin bridge
//! (`MainActivity.wrapSyncCredentialKey`/`unwrapSyncCredentialKey`)
//! before Rust persists it -- a standard key-wrapping pattern: the
//! wrapped bytes on disk are useless without the Keystore key that only
//! this device's OS holds, so they don't need `legere.db`'s own
//! protection the way the plaintext DEK would.
//!
//! The wrapped DEK lives in its own flat file under the app's data
//! directory (not the `settings` table `db::sync_config` otherwise uses
//! for everything local-only) -- deliberately, to keep this module
//! self-contained: `credential_vault`'s public `encrypt`/`decrypt` take
//! no `Connection`, so a `db::sync_config` dependency here would mean
//! either changing that shared signature just for one platform, or a
//! cross-module dependency back into `db::sync_config`'s private
//! settings accessors. [`init`] hands this module the one thing it
//! actually needs (the data directory), the same `OnceLock` pattern
//! `GLOBAL_APP_HANDLE`/`import_intent::CACHED_ACTIVITY` already establish
//! for other Android process-global state.
//!
//! The JNI bridge itself mirrors `import_intent.rs`'s cached-Activity
//! pattern exactly (a `GlobalRef` handed over once from
//! `MainActivity.onCreate`, reused for every later call) -- a separate
//! cache rather than sharing `import_intent`'s, matching this codebase's
//! existing convention of one cache per concern (see that module's own
//! doc comment). Not this crate's own third-party Android keyring
//! backend (`android-native-keyring-store`, available as a `keyring`
//! crate feature): that depends on `ndk-context`'s global state, which
//! `mobile_tls.rs`'s doc comment already documents, from direct
//! experience, as never populated in Tauri's Activity+WebView Android
//! runtime -- using it would mean bootstrapping that global anyway, plus
//! a second, older major version of the `jni` crate, for no benefit over
//! the small bridge here.

use std::path::PathBuf;
use std::sync::{Mutex as StdMutex, OnceLock};

use jni::EnvUnowned;
use jni::errors::LogErrorAndDefault;
use jni::objects::{JByteArray, JObject};
use jni::refs::Global;
use jni::signature::RuntimeMethodSignature;
use jni::vm::JavaVM;
use jni::{JValue, errors::Result as JniResult};
use rand::RngCore;
use rand::rngs::OsRng;

use super::{KeySource, VaultError};

/// A `GlobalRef` to the last-created `MainActivity`, handed over by
/// `cacheCredentialVaultActivity`. See this module's own doc comment for
/// why Rust needs this at all, and `import_intent.rs`'s identical
/// `CACHED_ACTIVITY` for why it's re-cached on every `onCreate` rather
/// than set once.
static CACHED_ACTIVITY: OnceLock<StdMutex<Option<(JavaVM, Global<JObject<'static>>)>>> =
    OnceLock::new();

/// This device's data directory, handed over once from `lib.rs`'s
/// `setup` closure (see [`init`]) -- the one piece of state this module
/// needs that isn't reachable through the JNI bridge itself.
static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// File name for the wrapped (Keystore-encrypted) DEK, under `data_dir`.
/// Not under `content/`/`media/` -- those are swept by `gc::sweep_orphaned_files`
/// against what the database references, and this file has no database
/// row of its own to be matched against.
const WRAPPED_KEY_FILE_NAME: &str = "remote_sync_credential_key.enc";

/// Called once from `lib.rs`'s `setup` closure, before anything in this
/// module can be used.
pub(crate) fn init(data_dir: PathBuf) {
    let _ = DATA_DIR.set(data_dir);
}

/// Called from `MainActivity.cacheCredentialVaultActivity` (itself called
/// from `onCreate`, alongside `cacheImportActivity`) -- hands this module
/// a `GlobalRef` to the Activity and its `JavaVM` so [`wrap`]/[`unwrap`]
/// can call back into `wrapSyncCredentialKey`/`unwrapSyncCredentialKey`
/// later.
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_com_ritvijsrivastava_legere_MainActivity_cacheCredentialVaultActivity<
    'local,
>(
    mut env: EnvUnowned<'local>,
    this: JObject<'local>,
) {
    env.with_env(|env| -> JniResult<()> {
        let vm = env.get_java_vm()?;
        let activity = env.new_global_ref(this)?;
        *CACHED_ACTIVITY
            .get_or_init(Default::default)
            .lock()
            .unwrap() = Some((vm, activity));
        Ok(())
    })
    .resolve::<LogErrorAndDefault>();
}

fn with_cached_activity<T>(
    f: impl FnOnce(&mut jni::Env, &JObject) -> JniResult<T>,
) -> Result<T, String> {
    let cell = CACHED_ACTIVITY
        .get()
        .ok_or("no Activity has registered itself yet")?;
    let guard = cell.lock().unwrap();
    let (vm, activity) = guard
        .as_ref()
        .ok_or("no Activity has registered itself yet")?;
    vm.attach_current_thread(|env| f(env, activity))
        .map_err(|error: jni::errors::Error| error.to_string())
}

/// Wraps `key` via `MainActivity.wrapSyncCredentialKey`.
fn wrap(key: &[u8; 32]) -> Result<Vec<u8>, String> {
    with_cached_activity(|env, activity| {
        let key_j = env.byte_array_from_slice(key)?;
        let sig = RuntimeMethodSignature::from_str("([B)[B")?;
        let result = env.call_method(
            activity,
            jni::strings::JNIString::from("wrapSyncCredentialKey"),
            sig.method_signature(),
            &[JValue::Object(&key_j)],
        )?;
        let wrapped_obj = result.l()?;
        if wrapped_obj.is_null() {
            // `wrapSyncCredentialKey` returning null means it swallowed a
            // real failure (see its own doc comment) -- nothing more
            // specific to report than "it didn't work".
            return Err(jni::errors::Error::JniCall(jni::errors::JniError::Unknown));
        }
        let wrapped_array = JByteArray::cast_local(env, wrapped_obj)?;
        env.convert_byte_array(wrapped_array)
    })
}

/// Unwraps `wrapped` via `MainActivity.unwrapSyncCredentialKey`.
fn unwrap(wrapped: &[u8]) -> Result<Vec<u8>, String> {
    with_cached_activity(|env, activity| {
        let wrapped_j = env.byte_array_from_slice(wrapped)?;
        let sig = RuntimeMethodSignature::from_str("([B)[B")?;
        let result = env.call_method(
            activity,
            jni::strings::JNIString::from("unwrapSyncCredentialKey"),
            sig.method_signature(),
            &[JValue::Object(&wrapped_j)],
        )?;
        let key_obj = result.l()?;
        if key_obj.is_null() {
            return Err(jni::errors::Error::JniCall(jni::errors::JniError::Unknown));
        }
        let key_array = JByteArray::cast_local(env, key_obj)?;
        env.convert_byte_array(key_array)
    })
}

fn wrapped_key_path() -> Option<PathBuf> {
    DATA_DIR.get().map(|dir| dir.join(WRAPPED_KEY_FILE_NAME))
}

pub struct AndroidKeySource;

impl KeySource for AndroidKeySource {
    fn get_or_create_key() -> Result<[u8; 32], VaultError> {
        let path = wrapped_key_path().ok_or(VaultError::KeyUnavailable)?;

        match std::fs::read(&path) {
            Ok(wrapped) => {
                let key = unwrap(&wrapped).map_err(|_| VaultError::KeyUnavailable)?;
                to_key_array(&key).ok_or(VaultError::KeyUnavailable)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // First use on this device: generate, wrap, and persist
                // the wrapped bytes *before* returning the key -- a
                // write failure here must not hand back a key nothing
                // durable backs (see the module doc's "standard
                // key-wrapping pattern" for why losing the wrapped copy
                // means losing the ability to ever decrypt again).
                let mut key = [0u8; 32];
                OsRng.fill_bytes(&mut key);
                let wrapped = wrap(&key).map_err(|_| VaultError::KeyUnavailable)?;
                std::fs::write(&path, &wrapped).map_err(|_| VaultError::KeyUnavailable)?;
                Ok(key)
            }
            // Any other I/O error (permissions, ...) is "unavailable
            // right now" -- not retried with a fresh `wrap`/overwrite,
            // same reasoning as `LinuxKeySource`'s own non-`NoEntry`
            // branch: the file may well still be there and readable
            // later, and overwriting it unconditionally risks orphaning
            // whatever was already encrypted with the key it wraps.
            Err(_) => Err(VaultError::KeyUnavailable),
        }
    }
}

/// `None` if `bytes` isn't exactly a 32-byte key -- defends against a
/// corrupted wrapped-key file or an unexpected Kotlin-side result length
/// without treating it as a fresh "no key yet" case.
fn to_key_array(bytes: &[u8]) -> Option<[u8; 32]> {
    let key: [u8; 32] = bytes.try_into().ok()?;
    Some(key)
}

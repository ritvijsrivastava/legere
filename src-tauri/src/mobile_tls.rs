//! On Android, `reqwest`'s rustls backend resolves to
//! `rustls-platform-verifier` for certificate verification, and that crate
//! requires an explicit JNI handoff before the first TLS handshake.
//!
//! The obvious approach — populating `ndk-context`'s global and reading it
//! back here — doesn't work: that crate is populated by `ndk-glue`/
//! `android-activity`, which is the `NativeActivity` model for GUI-less
//! Rust-first apps. Tauri's Android runtime is Activity+WebView based (see
//! `gen/android/app/src/main/java/.../generated/Rust.kt`) and never touches
//! `ndk-context`, so nothing populates it in a Tauri app and reading it
//! panics ("android context was not initialized") the instant it's called.
//!
//! Instead, `MainActivity.kt` declares `external fun initTls()` and calls
//! it from `onCreate` after `super.onCreate()` (see that file) — by then
//! `Rust.kt`'s `System.loadLibrary` has already run (triggered earlier in
//! the same `onCreate` chain, by the base `TauriActivity`/`WryActivity`),
//! so this symbol is resolvable. The calling `MainActivity` instance
//! doubles as the `Context` `init_with_env` needs.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_com_ritvijsrivastava_legere_MainActivity_initTls<'local>(
    mut env: jni::EnvUnowned<'local>,
    this: jni::objects::JObject<'local>,
) {
    match env
        .with_env(|env| rustls_platform_verifier::android::init_with_env(env, this))
        .into_outcome()
    {
        jni::Outcome::Ok(()) => {}
        jni::Outcome::Err(error) => {
            tracing::error!(%error, "failed to initialize rustls-platform-verifier's Android context")
        }
        jni::Outcome::Panic(_) => {
            tracing::error!("panicked while initializing rustls-platform-verifier's Android context")
        }
    }
}

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
//!
//! [`init_tls`] is also called directly (not through the JNI export below)
//! from `share_intent`'s native entrypoint: a shared URL can be the very
//! first thing this app process ever does — Android starts the process for
//! the WorkManager task alone, `MainActivity.onCreate` never runs — so
//! nothing else in that codepath has done this handoff yet either.
#[cfg(target_os = "android")]
pub(crate) fn init_tls<'local>(
    env: &mut jni::Env<'local>,
    context: jni::objects::JObject<'local>,
) -> jni::errors::Result<()> {
    rustls_platform_verifier::android::init_with_env(env, context)
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_com_ritvijsrivastava_legere_MainActivity_initTls<'local>(
    mut env: jni::EnvUnowned<'local>,
    this: jni::objects::JObject<'local>,
) {
    match env.with_env(|env| init_tls(env, this)).into_outcome() {
        jni::Outcome::Ok(()) => {}
        jni::Outcome::Err(error) => {
            tracing::error!(%error, "failed to initialize rustls-platform-verifier's Android context")
        }
        jni::Outcome::Panic(_) => {
            tracing::error!(
                "panicked while initializing rustls-platform-verifier's Android context"
            )
        }
    }
}

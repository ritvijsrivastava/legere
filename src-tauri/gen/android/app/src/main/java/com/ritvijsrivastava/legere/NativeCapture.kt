package com.ritvijsrivastava.legere

import android.content.Context

/**
 * JNI bridge into `share_intent.rs`'s standalone capture entrypoint \u2014
 * separate from [Rust] (the Tauri-generated bridge) because this call has
 * to work even when no Tauri runtime/`AppState` exists in this process
 * (see `share_intent.rs`'s module docs).
 *
 * `System.loadLibrary` is idempotent (a no-op if the current class
 * loader already loaded it), so it's safe to call here unconditionally
 * even when [Rust]'s own `init` block already loaded the same library
 * earlier in this process's life \u2014 and necessary when it hasn't: a share
 * can be the only thing this process ever does, with `MainActivity`
 * (and therefore [Rust]'s `init` block) never running at all.
 */
object NativeCapture {
    init {
        System.loadLibrary("legere_lib")
    }

    /**
     * Runs the full fetch\u2192extract\u2192sanitize\u2192localize\u2192store capture pipeline
     * for [url] against the same `legere.db`/`content/`/`media/` this
     * app's own Tauri runtime uses, and inserts the result as a new
     * article. Blocks the calling thread \u2014 call from a background thread
     * only (see [ShareWorker]).
     *
     * [context] is `applicationContext`, needed on the Rust side for the
     * same `rustls-platform-verifier` JNI handoff `MainActivity.initTls`
     * normally does. [dataDir] is `{filesDir}/legere`.
     *
     * Returns a JSON string: `{"ok":true,"title":"..."}` on success,
     * `{"ok":false,"error":"..."}` otherwise \u2014 never throws.
     */
    @JvmStatic
    external fun captureSharedUrl(context: Context, dataDir: String, url: String): String
}

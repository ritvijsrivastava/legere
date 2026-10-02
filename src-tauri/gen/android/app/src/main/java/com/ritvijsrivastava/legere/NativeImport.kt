package com.ritvijsrivastava.legere

import android.content.Context

/**
 * JNI bridge into `import_intent.rs`'s standalone/running-app import
 * entrypoint — mirrors [NativeCapture]/[NativeSync]'s exact shape (see
 * [NativeCapture]'s doc comment for why `System.loadLibrary` is repeated
 * here rather than assumed already done).
 */
object NativeImport {
    init {
        System.loadLibrary("legere_lib")
    }

    /**
     * Runs one Raindrop.io or article CSV import to completion, routed
     * through this app's own running `AppState` if one exists in this
     * process, or a standalone throwaway pool/client/runtime otherwise
     * (see `import_intent.rs`'s module docs). Blocks the calling thread;
     * call from a background thread only (see [ImportWorker]).
     *
     * [kind] is `"raindrop"` or `"article_csv"`. [context]/[dataDir] have
     * the same meaning as [NativeCapture.captureSharedUrl]'s identical
     * parameters (only actually used by the standalone fallback).
     *
     * Returns a JSON string: `{"ok":true}` on success,
     * `{"ok":false,"error":"..."}` otherwise — never throws. Per-row
     * failures within an otherwise-successful run aren't reported here;
     * see [pollImportProgress] for live counts instead.
     */
    @JvmStatic
    external fun runImport(
        context: Context,
        dataDir: String,
        kind: String,
        csvPath: String,
        concurrency: Int,
    ): String

    /**
     * The latest progress snapshot for whichever import is currently
     * running in this process, as JSON (`{"total":N,"processed":N,
     * "imported":N,"skipped_duplicate":N,"failed":N,"finished":bool,
     * "cancelled":bool,"error":"..."|null}`), or the literal string
     * `"null"` if none has started yet. [ImportWorker] polls this on a
     * timer to keep its notification's progress bar current while
     * [runImport] blocks a different thread.
     */
    @JvmStatic
    external fun pollImportProgress(): String

    /**
     * Flips whichever import's cooperative cancellation flag is
     * currently active — a no-op if nothing's running. Called from
     * [ImportWorker.onStopped].
     */
    @JvmStatic
    external fun cancelImport()
}

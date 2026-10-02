package com.ritvijsrivastava.legere

import android.content.Context

/**
 * JNI bridge into `remote_sync_intent.rs`'s standalone/running-app sync
 * entrypoint — mirrors [NativeCapture]'s exact shape (see that class's
 * doc comment for why `System.loadLibrary` is repeated here rather than
 * assumed already done).
 */
object NativeSync {
    init {
        System.loadLibrary("legere_lib")
    }

    /**
     * Runs one cross-device sync pass — routed through this app's own
     * running `AppState` if one exists in this process, or a standalone
     * throwaway pool/client/runtime otherwise (see
     * `remote_sync_intent.rs`'s module docs). Blocks the calling thread;
     * call from a background thread only (see [RemoteSyncWorker]).
     *
     * [context]/[dataDir] have the same meaning as
     * [NativeCapture.captureSharedUrl]'s identical parameters.
     *
     * Returns a JSON string: `{"ok":true,"pulled":N,"pushed":N}` on a
     * completed pass, `{"ok":true,"skipped":true}` if sync isn't
     * configured/enabled, `{"ok":false,"error":"..."}` otherwise — never
     * throws.
     */
    @JvmStatic
    external fun runRemoteSyncOnce(context: Context, dataDir: String): String

    /**
     * Reads the user's configured cross-device sync interval (hours),
     * defaulting to 6 if sync has never been configured or the DB can't
     * be read. Backs [RemoteSyncWorker.schedulePeriodic]'s periodic
     * `WorkManager` job cadence — see that function's doc comment for
     * why this is only re-read on app launch, not live.
     */
    @JvmStatic
    external fun getRemoteSyncIntervalHours(context: Context, dataDir: String): Int
}

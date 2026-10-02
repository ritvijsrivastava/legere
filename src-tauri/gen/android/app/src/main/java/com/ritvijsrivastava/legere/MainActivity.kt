package com.ritvijsrivastava.legere

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.view.textclassifier.TextClassifier
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.content.ContextCompat

class MainActivity : TauriActivity() {
  // Hands the JVM/Context to rustls-platform-verifier before any TLS
  // handshake can occur — see mobile_tls.rs for why this can't be done
  // from Rust's own startup path on Android. liblegere_lib.so is already
  // loaded by this point (Rust.kt's System.loadLibrary runs earlier in
  // this same onCreate chain, via the TauriActivity/WryActivity base).
  private external fun initTls()

  // Hands Rust a GlobalRef to this Activity instance so `import_intent.rs`
  // can later call back into `enqueueImportWork`/`cancelImportWork` below
  // — see that module's doc comment for why this reverse direction (Rust
  // calling Kotlin) needs this at all, unlike every other JNI bridge in
  // this codebase. Re-called every `onCreate`, same as `initTls()`, so a
  // recreated Activity never leaves a stale reference cached.
  private external fun cacheImportActivity()

  private var pendingImport: PendingImport? = null

  private data class PendingImport(val kind: String, val path: String, val concurrency: Int)

  private val requestNotificationPermissionForImport =
    registerForActivityResult(ActivityResultContracts.RequestPermission()) {
      // Proceed regardless of the answer — same accepted tradeoff as
      // ShareActivity's identical prompt: a denial just means the import
      // runs without a visible progress notification, not that it
      // doesn't run.
      startPendingImport()
    }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    initTls()
    cacheImportActivity()
    // See RemoteSyncWorker.schedulePeriodic's doc comment for why this
    // runs unconditionally rather than only when sync is enabled.
    RemoteSyncWorker.schedulePeriodic(applicationContext)
  }

  /**
   * Called from Rust (`import_intent::enqueue`) when the user starts a
   * Raindrop/article CSV import — hands the job to `ImportWorker` via
   * `WorkManager`, which keeps it running independently of this Activity
   * or even this process from here on. `kind` is `"raindrop"` or
   * `"article_csv"`.
   */
  fun enqueueImportWork(kind: String, path: String, concurrency: Int) {
    val needsPermissionPrompt = Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
      ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) !=
      PackageManager.PERMISSION_GRANTED
    pendingImport = PendingImport(kind, path, concurrency)
    if (needsPermissionPrompt) {
      requestNotificationPermissionForImport.launch(Manifest.permission.POST_NOTIFICATIONS)
    } else {
      startPendingImport()
    }
  }

  private fun startPendingImport() {
    val import = pendingImport ?: return
    pendingImport = null
    ImportWorker.enqueue(applicationContext, import.kind, import.path, import.concurrency)
  }

  /** Called from Rust (`import_intent::cancel`) — see that function's doc
   *  comment for why this always attempts the cancel unconditionally
   *  rather than first checking whether a job is actually running. */
  fun cancelImportWork() {
    ImportWorker.cancel(applicationContext)
  }

  /**
   * Called from Rust (`csv_source::read_csv_bytes`) to copy a SAF
   * `content://` URI — what the file picker returns on Android, good
   * only via [android.content.ContentResolver], never a real path — to
   * a real file under this app's own private storage. Returns `true` on
   * success; any failure (revoked permission, I/O error, malformed URI)
   * is swallowed to `false` rather than thrown, since the JNI caller has
   * no exception-safe way to let a Kotlin exception propagate back into
   * Rust.
   */
  fun copyContentUriToFile(uriString: String, destPath: String): Boolean {
    return try {
      val uri = android.net.Uri.parse(uriString)
      contentResolver.openInputStream(uri)?.use { input ->
        java.io.File(destPath).outputStream().use { output -> input.copyTo(output) }
      } != null
    } catch (e: Exception) {
      false
    }
  }

  // Every <input>/<textarea> in the WebView is backed by a real Android
  // EditText, which by default runs the system TextClassifier (on-device
  // entity detection for phone numbers/addresses/etc., feeding autofill
  // and "smart" text selection) on each edit. That classifier is known to
  // be pathologically slow on digit-heavy input and phone-dialing
  // punctuation ('*', '#') in particular, freezing the whole WebView—
  // including unrelated keystrokes like Backspace — for seconds at a
  // time (this is what made the library search box hang when searching
  // numbers or '*'). Legere has no use for on-device entity detection
  // anywhere in its UI, so disable it outright rather than trying to
  // dodge it per-input.
  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    webView.setTextClassifier(TextClassifier.NO_OP)
  }
}

package com.ritvijsrivastava.legere

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.view.textclassifier.TextClassifier
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.content.ContextCompat
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

class MainActivity : TauriActivity() {
  companion object {
    private const val ANDROID_KEYSTORE_PROVIDER = "AndroidKeyStore"
    // Scoped to this one use (wrapping the cross-device sync credential
    // DEK) rather than shared with any other feature that might someday
    // want its own Keystore key -- a leaked/rotated alias for one
    // purpose then can't affect another.
    private const val SYNC_CREDENTIAL_KEYSTORE_ALIAS = "legere_remote_sync_credential_key"
    private const val AES_GCM_TRANSFORMATION = "AES/GCM/NoPadding"
    // Matches `remote_sync::credential_vault`'s own `NONCE_LEN`/tag
    // length on the Rust side (AES-GCM's standard 96-bit nonce, 128-bit
    // tag) -- the two never interoperate directly (Rust never sees this
    // layer's plaintext DEK bytes until *after* a successful unwrap), but
    // keeping the envelope shape identical avoids two different
    // IV/tag-length conventions in the same feature for no reason.
    private const val GCM_IV_LENGTH_BYTES = 12
    private const val GCM_TAG_LENGTH_BITS = 128
  }

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

  // Hands Rust a GlobalRef to this Activity so `remote_sync::credential_vault::android`
  // can later call `wrapSyncCredentialKey`/`unwrapSyncCredentialKey` below —
  // same reverse-direction need and the same per-module re-caching
  // convention as `cacheImportActivity` above (see that property's own
  // comment); a separate cache rather than sharing one, since the two
  // are otherwise unrelated concerns.
  private external fun cacheCredentialVaultActivity()

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
    cacheCredentialVaultActivity()
    // See RemoteSyncWorker.schedulePeriodic's doc comment for why this
    // runs unconditionally rather than only when sync is enabled.
    // Off the UI thread: it reads the sync interval from SQLite via JNI,
    // and a UI thread stuck there past wry's 10s main-pipe timeout makes
    // tauri abort with "Could not find the webview runtime".
    val appContext = applicationContext
    Thread({
      RemoteSyncWorker.schedulePeriodic(appContext)
    }, "schedule-remote-sync").start()
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
   * Wraps (encrypts) [key] -- the 32-byte random DEK
   * `remote_sync::credential_vault::android::AndroidKeySource` generates
   * on first use -- with an Android Keystore-backed AES-256-GCM key
   * (generated here on first call, non-exportable: it never leaves the
   * Keystore, only a `Cipher` handle backed by it does). The *wrapped*
   * bytes (IV prepended to ciphertext) are what Rust persists to disk;
   * the Keystore key itself is this method's only route to ever
   * reading them back, via [unwrapSyncCredentialKey]. Returns null on
   * any failure (Keystore unavailable, ...) rather than throwing --
   * Rust has no exception-safe way to let a Kotlin exception cross the
   * JNI boundary, same convention as [copyContentUriToFile] above.
   */
  fun wrapSyncCredentialKey(key: ByteArray): ByteArray? {
    return try {
      val cipher = Cipher.getInstance(AES_GCM_TRANSFORMATION)
      cipher.init(Cipher.ENCRYPT_MODE, getOrCreateSyncCredentialKeystoreKey())
      cipher.iv + cipher.doFinal(key)
    } catch (e: Exception) {
      null
    }
  }

  /**
   * Reverses [wrapSyncCredentialKey]: unwraps [wrapped] (IV prepended to
   * ciphertext) back into the raw DEK bytes, using the same Keystore key.
   * Returns null on any failure (wrong/rotated Keystore key, corrupted
   * data, Keystore unavailable, truncated input), same convention as
   * [wrapSyncCredentialKey].
   */
  fun unwrapSyncCredentialKey(wrapped: ByteArray): ByteArray? {
    if (wrapped.size <= GCM_IV_LENGTH_BYTES) return null
    return try {
      val iv = wrapped.copyOfRange(0, GCM_IV_LENGTH_BYTES)
      val ciphertext = wrapped.copyOfRange(GCM_IV_LENGTH_BYTES, wrapped.size)
      val cipher = Cipher.getInstance(AES_GCM_TRANSFORMATION)
      cipher.init(
        Cipher.DECRYPT_MODE,
        getOrCreateSyncCredentialKeystoreKey(),
        GCMParameterSpec(GCM_TAG_LENGTH_BITS, iv)
      )
      cipher.doFinal(ciphertext)
    } catch (e: Exception) {
      null
    }
  }

  /**
   * This device's Keystore-backed AES-256-GCM key for wrapping the
   * cross-device sync credential DEK -- generated once (first call ever
   * on this device) and reused from then on; `KeyStore.getKey` returns
   * the same logical key across process restarts since it's the OS, not
   * this process, that owns its lifetime.
   */
  private fun getOrCreateSyncCredentialKeystoreKey(): SecretKey {
    val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE_PROVIDER)
    keyStore.load(null)
    (keyStore.getKey(SYNC_CREDENTIAL_KEYSTORE_ALIAS, null) as? SecretKey)?.let { return it }

    val keyGenerator =
      KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE_PROVIDER)
    val spec =
      KeyGenParameterSpec.Builder(
          SYNC_CREDENTIAL_KEYSTORE_ALIAS,
          KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
        )
        .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
        .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
        .setKeySize(256)
        .build()
    keyGenerator.init(spec)
    return keyGenerator.generateKey()
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

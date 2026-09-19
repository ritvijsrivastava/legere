package com.ritvijsrivastava.legere

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.content.ContextCompat
import androidx.work.Data
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.OutOfQuotaPolicy
import androidx.work.WorkManager
import java.util.regex.Pattern

/**
 * Trampoline for Android's "Share to Legere": this Activity is never
 * visible (see `Theme.legere.NoDisplay` in AndroidManifest.xml) and never
 * touches the Tauri webview/`MainActivity` at all. It only extracts a URL
 * from the incoming share, hands it to an expedited [ShareWorker] job,
 * and finishes \u2014 see ARCHITECTURE.md's "Share intent (Android)" section
 * for the full flow and why it's shaped this way.
 */
class ShareActivity : ComponentActivity() {

    private var pendingUrl: String? = null

    private val requestNotificationPermission =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) {
            // Proceed regardless of the answer \u2014 a denial just means the
            // save happens without a progress notification, not that it
            // doesn't happen.
            enqueueCaptureAndFinish()
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val sharedText = if (intent?.action == Intent.ACTION_SEND) {
            intent.getStringExtra(Intent.EXTRA_TEXT)
        } else {
            null
        }
        val url = sharedText?.let(::extractFirstUrl)
        if (url == null) {
            Toast.makeText(this, getString(R.string.share_no_link_found), Toast.LENGTH_SHORT).show()
            finish()
            return
        }
        pendingUrl = url

        val needsPermissionPrompt = Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) !=
            PackageManager.PERMISSION_GRANTED
        if (needsPermissionPrompt) {
            // A system permission dialog renders in its own window and is
            // shown fine even over this fully transparent Activity.
            requestNotificationPermission.launch(Manifest.permission.POST_NOTIFICATIONS)
        } else {
            enqueueCaptureAndFinish()
        }
    }

    private fun enqueueCaptureAndFinish() {
        val url = pendingUrl
        if (url == null) {
            finish()
            return
        }

        val notificationId = (System.currentTimeMillis() and 0x7fffffff).toInt()
        val data = Data.Builder()
            .putString(ShareWorker.KEY_URL, url)
            .putInt(ShareWorker.KEY_NOTIFICATION_ID, notificationId)
            .build()
        val request = OneTimeWorkRequestBuilder<ShareWorker>()
            .setInputData(data)
            // Best-effort immediacy: run right away with scheduling
            // priority, but fall back to ordinary (still-guaranteed,
            // just not immediate) background work rather than throwing
            // if the app has burned through its expedited-job quota.
            .setExpedited(OutOfQuotaPolicy.RUN_AS_NON_EXPEDITED_WORK_REQUEST)
            .build()
        WorkManager.getInstance(applicationContext).enqueue(request)

        finish()
    }

    private companion object {
        // Matches the first http(s) URL substring in shared text \u2014
        // ACTION_SEND's EXTRA_TEXT is very often a whole sentence
        // ("Check this out: https://example.com/article \u2014 via SomeApp"),
        // not a bare URL.
        val URL_PATTERN: Pattern = Pattern.compile("https?://\\S+")

        fun extractFirstUrl(text: String): String? {
            val matcher = URL_PATTERN.matcher(text)
            if (!matcher.find()) return null
            // Trailing punctuation a sentence would add right after the
            // link (closing sentence period, parenthesis, quote) isn't
            // part of the URL itself.
            return matcher.group().trimEnd('.', ',', ')', ']', '"', '\'', '!', '?')
        }
    }
}

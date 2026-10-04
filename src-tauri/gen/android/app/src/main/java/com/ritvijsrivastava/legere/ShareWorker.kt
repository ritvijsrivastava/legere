package com.ritvijsrivastava.legere

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.content.pm.ServiceInfo
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import androidx.work.ForegroundInfo
import androidx.work.Worker
import androidx.work.WorkerParameters
import java.io.File
import org.json.JSONObject

/**
 * Runs one shared-URL capture in the background, promoted to a foreground
 * service for its duration (via [getForegroundInfo]) so the OS treats it
 * as high-priority, user-visible work rather than something Doze/App
 * Standby can defer \u2014 exactly what `setExpedited` on the enqueuing
 * [androidx.work.OneTimeWorkRequest] (see `ShareActivity`) asks for.
 *
 * Owns one notification (`inputData`'s `KEY_NOTIFICATION_ID`) for the
 * whole job: posted as an ongoing/indeterminate "Saving..." by
 * `ShareActivity` the instant the share arrives (and by
 * [getForegroundInfo] on Android 11, the only version that calls it),
 * replaced with a final
 * saved/failed state in [doWork] once `NativeCapture.captureSharedUrl`
 * returns.
 */
class ShareWorker(appContext: Context, params: WorkerParameters) : Worker(appContext, params) {

    override fun doWork(): Result {
        val url = inputData.getString(KEY_URL)
        if (url.isNullOrBlank()) return Result.failure()
        val notificationId = inputData.getInt(KEY_NOTIFICATION_ID, DEFAULT_NOTIFICATION_ID)

        // Must match Tauri's own `app_local_data_dir()` resolution exactly
        // (see PathPlugin.kt's `getDataDir` — `Context.dataDir`, i.e.
        // `/data/user/0/<package>/`, *not* `filesDir`, i.e.
        // `/data/user/0/<package>/files/`) or this writes into a database
        // the running app never reads from.
        val dataDir = File(applicationContext.dataDir, "legere").absolutePath
        val resultJson = NativeCapture.captureSharedUrl(applicationContext, dataDir, url)
        val result = runCatching { JSONObject(resultJson) }.getOrNull()
        val succeeded = result?.optBoolean("ok") == true

        // `ok` only means the link was stored. If its capture failed, what's
        // stored is a link-only placeholder (see `capture_error` in
        // share_intent.rs) — say so, with the real reason, rather than
        // claiming a saved article.
        val captureError = result?.optString("capture_error")?.takeIf { it.isNotBlank() }

        val notification = if (succeeded && captureError != null) {
            buildNotification(
                title = applicationContext.getString(R.string.share_link_only_title),
                text = captureError,
                ongoing = false,
            )
        } else if (succeeded) {
            buildNotification(
                title = applicationContext.getString(R.string.share_saved_title),
                text = result?.optString("title")?.takeIf { it.isNotBlank() } ?: url,
                ongoing = false,
            )
        } else {
            val error = result?.optString("error")?.takeIf { it.isNotBlank() } ?: url
            buildNotification(
                title = applicationContext.getString(R.string.share_failed_title),
                text = error,
                ongoing = false,
            )
        }
        notifyIfPermitted(notificationId, notification)

        return if (succeeded) Result.success() else Result.failure()
    }

    override fun getForegroundInfo(): ForegroundInfo {
        val notificationId = inputData.getInt(KEY_NOTIFICATION_ID, DEFAULT_NOTIFICATION_ID)
        val notification = buildNotification(
            title = applicationContext.getString(R.string.share_saving_title),
            text = null,
            ongoing = true,
        )
        // (API 30 only — on Android 12+ expedited work never calls this;
        // `ShareActivity` posts the same notification itself instead.)
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            ForegroundInfo(notificationId, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
        } else {
            ForegroundInfo(notificationId, notification)
        }
    }

    private fun notifyIfPermitted(id: Int, notification: Notification) =
        notifyIfPermitted(applicationContext, id, notification)

    private fun buildNotification(title: String, text: String?, ongoing: Boolean): Notification =
        buildNotification(applicationContext, title, text, ongoing)

    companion object {
        const val KEY_URL = "url"
        const val KEY_NOTIFICATION_ID = "notification_id"
        const val CHANNEL_ID = "share_capture"
        const val DEFAULT_NOTIFICATION_ID = 1

        /**
         * How long the "Saving…" notification [ShareActivity] posts may stay
         * if nothing ever replaces it (process killed, job never ran) —
         * long enough for any real capture, short enough that a stuck one
         * clears itself.
         */
        const val SAVING_TIMEOUT_MS = 2 * 60 * 1000L

        /**
         * Posts [notification] unless the user denied POST_NOTIFICATIONS
         * (Android 13+) — the capture itself still runs and still succeeds
         * or fails; there's just no visible confirmation of it.
         */
        fun notifyIfPermitted(context: Context, id: Int, notification: Notification) {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
                ContextCompat.checkSelfPermission(context, android.Manifest.permission.POST_NOTIFICATIONS) !=
                PackageManager.PERMISSION_GRANTED
            ) {
                return
            }
            val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            manager.notify(id, notification)
        }

        fun buildNotification(
            context: Context,
            title: String,
            text: String?,
            ongoing: Boolean,
            timeoutMs: Long? = null,
        ): Notification {
            ensureChannel(context)
            val openApp = Intent(context, MainActivity::class.java).apply {
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
            }
            val contentIntent = PendingIntent.getActivity(
                context,
                0,
                openApp,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
            )
            return NotificationCompat.Builder(context, CHANNEL_ID)
                .setSmallIcon(R.drawable.ic_stat_legere)
                .setColor(ContextCompat.getColor(context, R.color.notification_accent))
                .setContentTitle(title)
                .setOngoing(ongoing)
                .setOnlyAlertOnce(true)
                .setAutoCancel(!ongoing)
                .setContentIntent(contentIntent)
                .apply {
                    text?.let {
                        setContentText(it)
                        // Errors are long; let the shade show all of it.
                        setStyle(NotificationCompat.BigTextStyle().bigText(it))
                    }
                    timeoutMs?.let { setTimeoutAfter(it) }
                    if (ongoing) setProgress(0, 0, true)
                }
                .build()
        }

        private fun ensureChannel(context: Context) {
            if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
            val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            if (manager.getNotificationChannel(CHANNEL_ID) != null) return
            val channel = NotificationChannel(
                CHANNEL_ID,
                context.getString(R.string.share_notification_channel_name),
                NotificationManager.IMPORTANCE_LOW,
            ).apply {
                description = context.getString(R.string.share_notification_channel_description)
            }
            manager.createNotificationChannel(channel)
        }
    }
}

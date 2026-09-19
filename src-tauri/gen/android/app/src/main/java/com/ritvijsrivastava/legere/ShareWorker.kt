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
 * [getForegroundInfo] when the job starts, replaced with a final
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

        val notification = if (succeeded) {
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
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            ForegroundInfo(notificationId, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
        } else {
            ForegroundInfo(notificationId, notification)
        }
    }

    private fun notifyIfPermitted(id: Int, notification: Notification) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            ContextCompat.checkSelfPermission(applicationContext, android.Manifest.permission.POST_NOTIFICATIONS) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            // The user denied the permission prompt (or it was never
            // asked) \u2014 the capture itself still ran and still succeeded
            // or failed; there's just no visible confirmation of it.
            return
        }
        val manager = applicationContext.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        manager.notify(id, notification)
    }

    private fun buildNotification(title: String, text: String?, ongoing: Boolean): Notification {
        ensureChannel()
        val openApp = Intent(applicationContext, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
        }
        val contentIntent = PendingIntent.getActivity(
            applicationContext,
            0,
            openApp,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        return NotificationCompat.Builder(applicationContext, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.stat_sys_download)
            .setContentTitle(title)
            .setOngoing(ongoing)
            .setOnlyAlertOnce(true)
            .setAutoCancel(!ongoing)
            .setContentIntent(contentIntent)
            .apply { text?.let(::setContentText) }
            .apply { if (ongoing) setProgress(0, 0, true) }
            .build()
    }

    private fun ensureChannel() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val manager = applicationContext.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (manager.getNotificationChannel(CHANNEL_ID) != null) return
        val channel = NotificationChannel(
            CHANNEL_ID,
            applicationContext.getString(R.string.share_notification_channel_name),
            NotificationManager.IMPORTANCE_LOW,
        ).apply {
            description = applicationContext.getString(R.string.share_notification_channel_description)
        }
        manager.createNotificationChannel(channel)
    }

    companion object {
        const val KEY_URL = "url"
        const val KEY_NOTIFICATION_ID = "notification_id"
        const val CHANNEL_ID = "share_capture"
        const val DEFAULT_NOTIFICATION_ID = 1
    }
}

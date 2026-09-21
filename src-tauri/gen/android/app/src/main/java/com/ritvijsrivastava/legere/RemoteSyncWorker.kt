package com.ritvijsrivastava.legere

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.Worker
import androidx.work.WorkerParameters
import java.io.File
import java.util.concurrent.TimeUnit
import org.json.JSONObject

/**
 * Runs one cross-device sync pass on `WorkManager`'s own hourly schedule
 * (see [schedulePeriodic]) — the actual mechanism that makes sync happen
 * in the background on Android at all, unlike desktop, where
 * `remote_sync::orchestrate::spawn_remote_sync_autosync`'s in-process
 * loop is enough on its own because the process just stays alive. Not
 * promoted to a foreground service (unlike [ShareWorker]'s one-time,
 * user-initiated job): `PeriodicWorkRequest` can't be expedited at all,
 * and a routine hourly background sync doesn't need the same
 * immediate-execution guarantee a user-triggered share does — it's fine
 * running as ordinary deferrable background work.
 *
 * Posts one notification per run: an indeterminate "Syncing..." while
 * [NativeSync.runRemoteSyncOnce] blocks this worker's thread, replaced
 * with a final synced/failed state once it returns. A "skipped" result
 * (sync isn't configured/enabled — expected whenever this fires after
 * the user has turned sync off, since the periodic job itself is never
 * explicitly cancelled, only ever a no-op from that point on) clears the
 * notification instead of showing anything, since nothing happened.
 */
class RemoteSyncWorker(appContext: Context, params: WorkerParameters) : Worker(appContext, params) {

    override fun doWork(): Result {
        notifyIfPermitted(
            NOTIFICATION_ID,
            buildNotification(
                title = applicationContext.getString(R.string.remote_sync_running_title),
                text = null,
                ongoing = true,
            ),
        )

        // Must match Tauri's own `app_local_data_dir()` resolution — see
        // `ShareWorker`'s identical comment on this exact path.
        val dataDir = File(applicationContext.dataDir, "legere").absolutePath
        val resultJson = NativeSync.runRemoteSyncOnce(applicationContext, dataDir)
        val result = runCatching { JSONObject(resultJson) }.getOrNull()
        val succeeded = result?.optBoolean("ok") == true
        val skipped = result?.optBoolean("skipped") == true

        if (skipped) {
            cancelNotification(NOTIFICATION_ID)
            return Result.success()
        }

        val notification = if (succeeded) {
            val pulled = result?.optInt("pulled", 0) ?: 0
            val pushed = result?.optInt("pushed", 0) ?: 0
            buildNotification(
                title = applicationContext.getString(R.string.remote_sync_finished_title),
                text = applicationContext.getString(R.string.remote_sync_finished_text, pulled, pushed),
                ongoing = false,
            )
        } else {
            val error = result?.optString("error")?.takeIf { it.isNotBlank() }
                ?: applicationContext.getString(R.string.remote_sync_unknown_error)
            buildNotification(
                title = applicationContext.getString(R.string.remote_sync_failed_title),
                text = error,
                ongoing = false,
            )
        }
        notifyIfPermitted(NOTIFICATION_ID, notification)

        // A failed pass is retried on WorkManager's own backoff schedule
        // rather than waiting for the next full hourly tick — the same
        // "an errored source recovers on its own" spirit
        // `sync::sync_all_sources` already applies to RSS sources.
        return if (succeeded) Result.success() else Result.retry()
    }

    private fun cancelNotification(id: Int) {
        val manager = applicationContext.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        manager.cancel(id)
    }

    private fun notifyIfPermitted(id: Int, notification: Notification) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            ContextCompat.checkSelfPermission(applicationContext, android.Manifest.permission.POST_NOTIFICATIONS) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            // No Activity is on screen for a periodic background job to
            // prompt for this permission from — if it was never granted
            // (e.g. the user has never shared an article either), sync
            // still runs, just silently. Same trade `ShareWorker` accepts.
            return
        }
        ensureChannel()
        val manager = applicationContext.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        manager.notify(id, notification)
    }

    private fun buildNotification(title: String, text: String?, ongoing: Boolean): Notification {
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
            .setSmallIcon(android.R.drawable.stat_notify_sync)
            .setContentTitle(title)
            .setOngoing(ongoing)
            .setOnlyAlertOnce(true)
            .setAutoCancel(!ongoing)
            .setContentIntent(contentIntent)
            .apply { text?.let(::setContentText) }
            // Indeterminate only — a real percentage bar would need a
            // JNI progress callback from mid-`run_sync` back into this
            // worker, which doesn't exist yet (see
            // `remote_sync::engine::SyncProgress`, currently only wired
            // to the desktop/frontend `remote-sync:progress` event).
            .apply { if (ongoing) setProgress(0, 0, true) }
            .build()
    }

    private fun ensureChannel() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val manager = applicationContext.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (manager.getNotificationChannel(CHANNEL_ID) != null) return
        val channel = NotificationChannel(
            CHANNEL_ID,
            applicationContext.getString(R.string.remote_sync_notification_channel_name),
            NotificationManager.IMPORTANCE_LOW,
        ).apply {
            description = applicationContext.getString(R.string.remote_sync_notification_channel_description)
        }
        manager.createNotificationChannel(channel)
    }

    companion object {
        const val CHANNEL_ID = "remote_sync"
        const val NOTIFICATION_ID = 2
        private const val UNIQUE_WORK_NAME = "remote_sync_periodic"

        /**
         * Schedules the periodic sync job if it isn't already scheduled
         * (`KEEP` — repeat calls, e.g. every app launch, are cheap
         * no-ops once it exists). Called unconditionally from
         * `MainActivity.onCreate`, regardless of whether sync is
         * currently configured/enabled: [doWork] itself checks that on
         * the Rust side every time it fires and no-ops if not, which is
         * simpler than reaching back into WorkManager from Rust to
         * schedule/cancel every time the setting changes — the accepted
         * cost is an hourly no-op wakeup while sync is off, not a
         * correctness issue.
         */
        fun schedulePeriodic(context: Context) {
            val request = PeriodicWorkRequestBuilder<RemoteSyncWorker>(1, TimeUnit.HOURS).build()
            WorkManager.getInstance(context).enqueueUniquePeriodicWork(
                UNIQUE_WORK_NAME,
                ExistingPeriodicWorkPolicy.KEEP,
                request,
            )
        }
    }
}

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
import androidx.work.Data
import androidx.work.ExistingWorkPolicy
import androidx.work.ForegroundInfo
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.OutOfQuotaPolicy
import androidx.work.WorkManager
import androidx.work.Worker
import androidx.work.WorkerParameters
import java.io.File
import java.util.concurrent.atomic.AtomicReference
import org.json.JSONObject

/**
 * Runs one Raindrop.io/article CSV import in the background, promoted to
 * a foreground service for its duration (see [getForegroundInfo]) so the
 * OS treats it as high-priority work rather than something Doze/App
 * Standby defers — same reasoning as [ShareWorker]. This is the actual
 * mechanism that makes an import survive this app's own process dying
 * while backgrounded; see `import_intent.rs`'s module docs for the full
 * picture, including why *starting* this job needs Rust to call back
 * into [MainActivity] rather than the more usual Kotlin-calls-Rust
 * direction every other `Native*` bridge in this codebase uses.
 *
 * Unlike [ShareWorker]'s single on/off notification state, an import can
 * run for minutes over thousands of rows, so this shows a real
 * determinate progress bar: [NativeImport.runImport] blocks a dedicated
 * [Thread] (not this `doWork()` thread itself, which needs to stay free
 * to poll) while [doWork] polls [NativeImport.pollImportProgress] on a
 * timer and updates both the foreground notification and `setProgress`
 * bar from it.
 */
class ImportWorker(appContext: Context, params: WorkerParameters) : Worker(appContext, params) {

    override fun doWork(): Result {
        val kind = inputData.getString(KEY_KIND)
        val csvPath = inputData.getString(KEY_CSV_PATH)
        if (kind.isNullOrBlank() || csvPath.isNullOrBlank()) return Result.failure()
        val concurrency = inputData.getInt(KEY_CONCURRENCY, DEFAULT_CONCURRENCY)

        // Must match Tauri's own `app_local_data_dir()` resolution — see
        // `ShareWorker`'s identical comment on this exact path.
        val dataDir = File(applicationContext.dataDir, "legere").absolutePath

        val resultJson = AtomicReference<String?>(null)
        val importThread = Thread {
            resultJson.set(NativeImport.runImport(applicationContext, dataDir, kind, csvPath, concurrency))
        }
        importThread.start()

        var lastNotifiedProcessed = -1
        while (importThread.isAlive) {
            val snapshot = currentSnapshot()
            val processed = snapshot?.optInt("processed", 0) ?: -1
            if (snapshot != null && processed != lastNotifiedProcessed) {
                lastNotifiedProcessed = processed
                updateForegroundNotification(kind, snapshot)
            }
            Thread.sleep(POLL_INTERVAL_MS)
        }
        importThread.join()

        val finalResult = runCatching { JSONObject(resultJson.get() ?: "") }.getOrNull()
        val succeeded = finalResult?.optBoolean("ok") == true
        val finalSnapshot = currentSnapshot()

        val notification = if (succeeded) {
            buildFinishedNotification(kind, finalSnapshot)
        } else {
            val error = finalResult?.optString("error")?.takeIf { it.isNotBlank() }
                ?: applicationContext.getString(R.string.import_unknown_error)
            buildFailedNotification(error)
        }
        notifyIfPermitted(NOTIFICATION_ID, notification)

        return if (succeeded) Result.success() else Result.failure()
    }

    override fun onStopped() {
        super.onStopped()
        NativeImport.cancelImport()
    }

    override fun getForegroundInfo(): ForegroundInfo {
        val kind = inputData.getString(KEY_KIND) ?: ""
        val notification = buildRunningNotification(kind, currentSnapshot())
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            ForegroundInfo(NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
        } else {
            ForegroundInfo(NOTIFICATION_ID, notification)
        }
    }

    private fun currentSnapshot(): JSONObject? {
        val json = NativeImport.pollImportProgress()
        return runCatching { JSONObject(json) }.getOrNull()
    }

    private fun updateForegroundNotification(kind: String, snapshot: JSONObject) {
        ensureChannel()
        val manager = applicationContext.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        notifyIfPermitted(NOTIFICATION_ID, buildRunningNotification(kind, snapshot), manager)
    }

    private fun buildRunningNotification(kind: String, snapshot: JSONObject?): Notification {
        val total = snapshot?.optInt("total", 0) ?: 0
        val processed = snapshot?.optInt("processed", 0) ?: 0
        val title = applicationContext.getString(importingTitleRes(kind))
        return baseBuilder(title, ongoing = true)
            .apply {
                if (total > 0) {
                    setContentText(
                        applicationContext.getString(R.string.import_progress_text, processed, total),
                    )
                    setProgress(total, processed, false)
                } else {
                    setProgress(0, 0, true)
                }
            }
            .build()
    }

    private fun buildFinishedNotification(kind: String, snapshot: JSONObject?): Notification {
        val cancelled = snapshot?.optBoolean("cancelled", false) == true
        val imported = snapshot?.optInt("imported", 0) ?: 0
        val title = applicationContext.getString(
            if (cancelled) R.string.import_cancelled_title else importedTitleRes(kind),
        )
        return baseBuilder(title, ongoing = false)
            .setContentText(applicationContext.getString(R.string.import_finished_text, imported))
            .build()
    }

    private fun buildFailedNotification(error: String): Notification {
        return baseBuilder(applicationContext.getString(R.string.import_failed_title), ongoing = false)
            .setContentText(error)
            .build()
    }

    private fun baseBuilder(title: String, ongoing: Boolean): NotificationCompat.Builder {
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
    }

    private fun importingTitleRes(kind: String) =
        if (kind == KIND_ARTICLE_CSV) R.string.import_articles_running_title else R.string.import_raindrop_running_title

    private fun importedTitleRes(kind: String) =
        if (kind == KIND_ARTICLE_CSV) R.string.import_articles_finished_title else R.string.import_raindrop_finished_title

    private fun notifyIfPermitted(
        id: Int,
        notification: Notification,
        manager: NotificationManager = applicationContext.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager,
    ) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            ContextCompat.checkSelfPermission(applicationContext, android.Manifest.permission.POST_NOTIFICATIONS) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            // The import itself still runs — there's just no visible
            // progress/confirmation of it. Same accepted tradeoff as
            // ShareWorker/RemoteSyncWorker.
            return
        }
        manager.notify(id, notification)
    }

    private fun ensureChannel() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val manager = applicationContext.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (manager.getNotificationChannel(CHANNEL_ID) != null) return
        val channel = NotificationChannel(
            CHANNEL_ID,
            applicationContext.getString(R.string.import_notification_channel_name),
            NotificationManager.IMPORTANCE_LOW,
        ).apply {
            description = applicationContext.getString(R.string.import_notification_channel_description)
        }
        manager.createNotificationChannel(channel)
    }

    companion object {
        const val KEY_KIND = "kind"
        const val KEY_CSV_PATH = "csv_path"
        const val KEY_CONCURRENCY = "concurrency"
        const val KIND_ARTICLE_CSV = "article_csv"
        const val CHANNEL_ID = "import"
        const val NOTIFICATION_ID = 3
        const val DEFAULT_CONCURRENCY = 5
        const val POLL_INTERVAL_MS = 750L
        private const val UNIQUE_WORK_NAME = "legere_import"

        /**
         * Schedules [ImportWorker] — called from `MainActivity.enqueueImportWork`,
         * itself called from Rust (`import_intent::enqueue`) when the user
         * starts an import. `KEEP` rather than `REPLACE`: this is also the
         * single-import-at-a-time guard on Android (see `import_intent.rs`'s
         * module docs for why it's this, not `AppState::import_cancel`,
         * that actually enforces that here) — a second start request
         * while one is already running/enqueued is silently dropped rather
         * than restarting or running alongside it.
         */
        fun enqueue(context: Context, kind: String, csvPath: String, concurrency: Int) {
            val data = Data.Builder()
                .putString(KEY_KIND, kind)
                .putString(KEY_CSV_PATH, csvPath)
                .putInt(KEY_CONCURRENCY, concurrency)
                .build()
            val request = OneTimeWorkRequestBuilder<ImportWorker>()
                .setInputData(data)
                .setExpedited(OutOfQuotaPolicy.RUN_AS_NON_EXPEDITED_WORK_REQUEST)
                .build()
            WorkManager.getInstance(context).enqueueUniqueWork(
                UNIQUE_WORK_NAME,
                ExistingWorkPolicy.KEEP,
                request,
            )
        }

        /** Cancels the running/enqueued import job, if any — a no-op
         *  otherwise. See `import_intent::cancel`'s doc comment. */
        fun cancel(context: Context) {
            WorkManager.getInstance(context).cancelUniqueWork(UNIQUE_WORK_NAME)
        }
    }
}

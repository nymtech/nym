package net.nymtech.nyms5

import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.os.Build
import android.util.Log
import androidx.annotation.RequiresApi
import androidx.core.app.NotificationCompat
import androidx.work.CoroutineWorker
import androidx.work.ForegroundInfo
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import kotlinx.coroutines.delay

const val notificationId = 2001

class ProxyWorker(context: Context, parameters: WorkerParameters) :
    CoroutineWorker(context, parameters) {
    companion object {
        val name = "nymS5ProxyWorker"
    }

    private val tag = "proxyWorker"

    private val channelId =
        applicationContext.getString(R.string.notification_channel_id)

    private val notificationManager =
        context.getSystemService(Context.NOTIFICATION_SERVICE) as
                NotificationManager

    @RequiresApi(Build.VERSION_CODES.O)
    override suspend fun doWork(): Result {
        setForeground(createForegroundInfo())
        return try {
            Log.d(tag, "starting work")
            Socks5().start()
            // TODO as a temp workaround use this dirty loop
            //  as `start` lib call is not blocking and will returns
            //  after having spawned the proxy connection in another thread
            while (true) {
                delay(10)
            }
            Log.d(tag, "work finished")
            Result.success()
        } catch (throwable: Throwable) {
            Log.e(tag, "error: ${throwable.message}")
            Result.failure()
        }
    }

    // Creates an instance of ForegroundInfo which can be used to update the
    // ongoing notification.
    @RequiresApi(Build.VERSION_CODES.O)
    private fun createForegroundInfo(): ForegroundInfo {
        val title = applicationContext.getString(R.string.notification_title)
        val cancel = applicationContext.getString(R.string.stop_proxy)
        // This PendingIntent can be used to cancel the worker
        val intent = WorkManager.getInstance(applicationContext)
            .createCancelPendingIntent(getId())

        // Create a Notification channel if necessary
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            createChannel()
        }

        val notification = NotificationCompat.Builder(applicationContext, channelId)
            .setContentTitle(title)
            .setTicker(title)
            .setContentText("Nym socks5 proxy running")
            .setSmallIcon(android.R.drawable.ic_secure)
            .setOngoing(true)
            // Add the cancel action to the notification which can
            // be used to cancel the worker
            .addAction(android.R.drawable.ic_delete, cancel, intent)
            .build()

        return ForegroundInfo(notificationId, notification)
    }

    @RequiresApi(Build.VERSION_CODES.O)
    private fun createChannel() {
        notificationManager.createNotificationChannel(
            NotificationChannel(
                channelId,
                "nym proxy",
                NotificationManager.IMPORTANCE_HIGH
            )
        )
    }
}

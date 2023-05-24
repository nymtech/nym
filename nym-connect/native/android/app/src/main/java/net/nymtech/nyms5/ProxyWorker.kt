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
import fuel.Fuel
import fuel.get
import kotlinx.serialization.Serializable
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.json.Json
import kotlin.random.Random

class ProxyWorker(context: Context, parameters: WorkerParameters) :
    CoroutineWorker(context, parameters) {
    companion object {
        const val name = "nymS5ProxyWorker"
    }

    private val tag = "proxyWorker"

    private val spUrl = context.getString(R.string.sp_url)

    private val defaultSp = context.getString(R.string.default_sp)

    private val channelId =
        applicationContext.getString(R.string.notification_channel_id)

    private val notificationId = 2001

    private val notificationManager =
        context.getSystemService(Context.NOTIFICATION_SERVICE) as
                NotificationManager

    @Serializable
    data class SPData(
        val service_provider_client_id: String,
        val gateway_identity_key: String,
        val routing_score: Float,
        val ip_address: String
    )

    @Serializable
    data class SPListData(val items: List<SPData>)

    private val json = Json { ignoreUnknownKeys = true }

    @RequiresApi(Build.VERSION_CODES.O)
    override suspend fun doWork(): Result {
        setForeground(createForegroundInfo())
        return try {
            Log.d(tag, "starting work")

            var serviceProvider: String? = null
            // fetch the SP list and select a random one
            try {
                val res = Fuel.get(spUrl)
                if (res.statusCode == 200) {
                    val spJson = json.decodeFromString<SPListData>(res.body)
                    serviceProvider =
                        Random.nextInt(until = spJson.items.size)
                            .let { spJson.items[it].service_provider_client_id }
                    Log.d(tag, "selected service provider: $serviceProvider")
                } else {
                    Log.w(tag, "failed to fetch the service providers list: $res.statusCode")
                    Log.w(tag, "using a default service provider $defaultSp")
                }
            } catch (e: Throwable) {
                Log.e(tag, "an error occurred while fetching the service providers list: $e")
                Log.w(tag, "using a default service provider $defaultSp")
            }

            Socks5().start(serviceProvider?: defaultSp)
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
            .createCancelPendingIntent(id)

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
                applicationContext.getString(R.string.notification_channel_name),
                NotificationManager.IMPORTANCE_HIGH
            )
        )
    }
}

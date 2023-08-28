package net.nymtech.nyms5

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Build
import android.util.Log
import androidx.annotation.RequiresApi
import androidx.core.app.NotificationCompat
import androidx.work.CoroutineWorker
import androidx.work.ForegroundInfo
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import androidx.work.workDataOf
import com.github.kittinunf.fuel.Fuel
import io.sentry.Sentry
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.json.Json
import java.util.UUID
import kotlin.random.Random

class ProxyWorker(
    context: Context,
    parameters: WorkerParameters,
    private val nymProxy: NymProxy
) :
    CoroutineWorker(context, parameters) {
    companion object Work {
        const val name = "nymS5ProxyWorker"
        const val workTag = "nymProxy"
        // it is very important to use a static UUID in order to allow WorkManager
        // handling the proxy work as a unique work
        val workId: UUID = UUID.fromString("cc785aa4-5775-4bf0-b870-39645e35e660")

        const val State = "State"

        enum class Status {
            DISCONNECTED,
            STARTING,
            CONNECTED
        }
    }

    private val tag = "proxyWorker"

    private val pingRate = 1000L

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

    private val onStartCb = object {
        fun onStart() {
            Log.d(tag, "⚡ ON START callback")
            setProgressAsync(workDataOf(State to Status.CONNECTED.name))
        }
    }

    private val onStopCb = object {
        fun onStop() {
            Log.d(tag, "⚡ ON STOP callback")
            setProgressAsync(workDataOf(State to Status.DISCONNECTED.name))
        }
    }

    @RequiresApi(Build.VERSION_CODES.O)
    override suspend fun doWork(): Result {
        setProgress(workDataOf(State to Status.STARTING.name))

        // set this work as a long running worker
        // see https://developer.android.com/guide/background/persistent/how-to/long-running
        // `setForeground` can fail
        // see https://developer.android.com/guide/background/persistent/getting-started/define-work#coroutineworker
        try {
            setForeground(createForegroundInfo())
        } catch (e: Throwable) {
            Log.w(
                tag,
                "failed to make the work run in the context of a foreground service"
            )
            Sentry.captureException(e)
        }

        return try {
            Log.d(tag, "starting work")

            var serviceProvider: String? = null
            // fetch the SP list and select a random one
            try {
                val (_, response, result) = Fuel.get(spUrl).responseString()
                result.fold(
                    { data ->
                        val spJson = json.decodeFromString<SPListData>(data)
                        serviceProvider =
                            Random.nextInt(until = spJson.items.size)
                                .let { spJson.items[it].service_provider_client_id }
                        Log.d(tag, "selected service provider: $serviceProvider")
                    },
                    { error ->
                        Log.w(
                            tag,
                            "failed to fetch the service provider list: ${response.statusCode} ${error.message}"
                        )
                        Log.w(tag, "using a default service provider $defaultSp")
                    }
                )
            } catch (e: Throwable) {
                Log.e(
                    tag,
                    "an error occurred while fetching the service providers list: $e"
                )
                Sentry.captureException(e)
                Log.w(tag, "using a default service provider $defaultSp")
            }

            withContext(Dispatchers.IO) {
                val pingJob = launch {
                    // this job will get automatically killed by the WorkManager once
                    // the job has been terminated, so it's safe to use `while (true)`
                    while (true) {
                        nymProxy.ping()
                        delay(pingRate)
                    }
                }

                val proxyJob = launch {
                    nymProxy.start(serviceProvider ?: defaultSp, onStartCb, onStopCb)
                }

                // wait for the underlying call to `startClient` to be released which means
                // the connection has been terminated (`startClient` is a blocking call)
                proxyJob.join()
                // stop pinging
                pingJob.cancel()
            }

            // the state should be already set to DISCONNECTED at this point
            // but for the sake of it, reset it
            setProgress(workDataOf(State to Status.DISCONNECTED.name))
            Log.i(tag, "work finished")
            Result.success()
        } catch (e: Throwable) {
            Log.e(tag, "error: ${e.message}")
            Sentry.captureException(e)
            Result.failure()
        }
    }

    private fun createNotification(): Notification {
        val title = applicationContext.getString(R.string.notification_title)
        val cancel = applicationContext.getString(R.string.notification_action_stop)
        val content = applicationContext.getString(R.string.notification_content)
        // this pending intent is used to cancel the worker
        // TODO instead of using this intent to cancel the work
        //  use a custom intent to call `nymProxy.stopClient`
        val stopPendingIntent = WorkManager.getInstance(applicationContext)
            .createCancelPendingIntent(id)

        // this intent is used for the notification's tap action
        // on tap → show to the main activity
        val tapIntent =
            Intent(applicationContext, MainActivity::class.java).apply {
                flags =
                    Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_REORDER_TO_FRONT
            }
        val tapPendingIntent: PendingIntent = PendingIntent.getActivity(
            applicationContext,
            0,
            tapIntent,
            PendingIntent.FLAG_IMMUTABLE
        )

        return NotificationCompat.Builder(applicationContext, channelId)
            .setContentTitle(title)
            .setContentText(content)
            .setSmallIcon(R.drawable.shield_24)
            .setOngoing(true)
            .setContentIntent(tapPendingIntent)
            .addAction(android.R.drawable.ic_delete, cancel, stopPendingIntent)
            .build()
    }

    // Creates an instance of ForegroundInfo which can be used to update the
    // ongoing notification.
    @RequiresApi(Build.VERSION_CODES.O)
    private fun createForegroundInfo(): ForegroundInfo {
        Log.d(tag, "__createForegroundInfo")

        // Create a Notification channel if necessary
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            createChannel()
        }

        return ForegroundInfo(notificationId, createNotification())
    }

    // TODO without this override, under Android 11 the app crashes
    //  see https://developer.android.com/guide/background/persistent/getting-started/define-work#coroutineworker
    //  override doesn't seem to be a problem for newer versions
    override suspend fun getForegroundInfo(): ForegroundInfo {
        Log.d(tag, "__getForegroundInfo")

        // Create a Notification channel if necessary
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            createChannel()
        }
        return ForegroundInfo(notificationId, createNotification())
    }

    @RequiresApi(Build.VERSION_CODES.O)
    private fun createChannel() {
        Log.d(tag, "creating notification channel")
        notificationManager.createNotificationChannel(
            NotificationChannel(
                channelId,
                applicationContext.getString(R.string.notification_channel_name),
                NotificationManager.IMPORTANCE_DEFAULT
            )
        )
    }
}

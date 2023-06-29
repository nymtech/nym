package net.nymtech.nyms5

import android.content.Context
import androidx.work.ListenableWorker
import androidx.work.WorkerFactory
import androidx.work.WorkerParameters

class CustomWorkerFactory(private val nymProxy: NymProxy) : WorkerFactory() {

    override fun createWorker(
        appContext: Context,
        workerClassName: String,
        workerParameters: WorkerParameters
    ): ListenableWorker? {

        return when (workerClassName) {
            ProxyWorker::class.java.name ->
                ProxyWorker(appContext, workerParameters, nymProxy)

            else ->
                // Return null, so that the base class can delegate to the default WorkerFactory.
                null
        }

    }
}
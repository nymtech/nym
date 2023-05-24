package net.nymtech.nyms5

import android.app.Application
import android.util.Log
import androidx.work.Configuration
import androidx.work.DelegatingWorkerFactory

class App : Application(), Configuration.Provider {
    companion object {
        // NymProxy singleton (unique instance)
        val nymProxy = NymProxy()
    }

    private val tag = "App"

    override fun getWorkManagerConfiguration(): Configuration {
        val workerFactory = DelegatingWorkerFactory()
        // pass in the NymProxy class instance
        workerFactory.addFactory(CustomWorkerFactory(nymProxy))

        Log.d(tag, "using a custom configuration for WorkManager")
        return Configuration.Builder().setWorkerFactory(workerFactory).build()
    }
}
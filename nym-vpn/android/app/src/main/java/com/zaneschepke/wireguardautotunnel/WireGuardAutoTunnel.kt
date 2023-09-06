package net.nymtech.nymconnect

import android.app.Application
import android.content.Context
import android.content.pm.PackageManager
import net.nymtech.nymconnect.repository.Repository
import net.nymtech.nymconnect.service.tunnel.model.Settings
import dagger.hilt.android.HiltAndroidApp
import javax.inject.Inject

@HiltAndroidApp
class WireGuardAutoTunnel : Application() {

    @Inject
    lateinit var settingsRepo : Repository<Settings>

    override fun onCreate() {
        super.onCreate()
        settingsRepo.init()
    }

    companion object {
        fun isRunningOnAndroidTv(context : Context) : Boolean {
            return context.packageManager.hasSystemFeature(PackageManager.FEATURE_LEANBACK)
        }
    }
}
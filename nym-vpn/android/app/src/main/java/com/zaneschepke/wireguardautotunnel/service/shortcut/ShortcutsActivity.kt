package net.nymtech.nymconnect.service.shortcut

import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import net.nymtech.nymconnect.R
import net.nymtech.nymconnect.service.foreground.Action
import net.nymtech.nymconnect.service.foreground.ServiceManager
import net.nymtech.nymconnect.service.foreground.WireGuardTunnelService
import dagger.hilt.android.AndroidEntryPoint

@AndroidEntryPoint
class ShortcutsActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if(intent.getStringExtra(ShortcutsManager.CLASS_NAME_EXTRA_KEY)
            .equals(WireGuardTunnelService::class.java.name)) {
            intent.getStringExtra(getString(R.string.tunnel_extras_key))?.let {
                ServiceManager.toggleWatcherService(this, it)
            }
            when(intent.action){
                Action.STOP.name -> ServiceManager.stopVpnService(this)
                Action.START.name -> intent.getStringExtra(getString(R.string.tunnel_extras_key))
                    ?.let { ServiceManager.startVpnService(this, it) }
            }
        }
        finish()
    }
}
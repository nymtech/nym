package net.nymtech.nymconnect.service.tunnel

import com.wireguard.android.backend.Statistics
import com.wireguard.android.backend.Tunnel
import com.wireguard.crypto.Key
import net.nymtech.nymconnect.service.tunnel.model.TunnelConfig
import kotlinx.coroutines.flow.SharedFlow

interface VpnService : Tunnel {
    suspend fun startTunnel(tunnelConfig : TunnelConfig) : Tunnel.State
    suspend fun stopTunnel()
    val state : SharedFlow<Tunnel.State>
    val tunnelName : SharedFlow<String>
    val statistics : SharedFlow<Statistics>
    val lastHandshake : SharedFlow<Map<Key,Long>>
    val handshakeStatus : SharedFlow<HandshakeStatus>
    fun getState() : Tunnel.State
}
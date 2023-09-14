package net.nymtech.nymconnect.module

import android.content.Context
import com.wireguard.android.backend.Backend
import com.wireguard.android.backend.GoBackend
import net.nymtech.nymconnect.service.tunnel.VpnService
import net.nymtech.nymconnect.service.tunnel.WireGuardTunnel
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.android.qualifiers.ApplicationContext
import dagger.hilt.components.SingletonComponent
import javax.inject.Singleton

@Module
@InstallIn(SingletonComponent::class)
class TunnelModule {

    @Provides
    @Singleton
    fun provideBackend(@ApplicationContext context : Context) : Backend {
        return GoBackend(context)
    }

    @Provides
    @Singleton
    fun provideVpnService(backend: Backend) : VpnService {
        return WireGuardTunnel(backend)
    }

}
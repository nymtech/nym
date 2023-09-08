package net.nymtech.nymconnect.module

import net.nymtech.nymconnect.repository.Repository
import net.nymtech.nymconnect.repository.SettingsBox
import net.nymtech.nymconnect.repository.TunnelBox
import net.nymtech.nymconnect.service.tunnel.model.Settings
import net.nymtech.nymconnect.service.tunnel.model.TunnelConfig
import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import javax.inject.Singleton

@Module
@InstallIn(SingletonComponent::class)
abstract class RepositoryModule {

    @Binds
    @Singleton
    abstract fun provideSettingsRepository(settingsBox: SettingsBox) : Repository<Settings>

    @Binds
    @Singleton
    abstract fun provideTunnelRepository(tunnelBox: TunnelBox) : Repository<TunnelConfig>
}
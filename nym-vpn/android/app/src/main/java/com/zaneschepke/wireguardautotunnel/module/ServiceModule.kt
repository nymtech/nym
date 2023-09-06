package net.nymtech.nymconnect.module

import net.nymtech.nymconnect.service.network.MobileDataService
import net.nymtech.nymconnect.service.network.NetworkService
import net.nymtech.nymconnect.service.network.WifiService
import net.nymtech.nymconnect.service.notification.NotificationService
import net.nymtech.nymconnect.service.notification.WireGuardNotification
import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.android.components.ServiceComponent
import dagger.hilt.android.scopes.ServiceScoped

@Module
@InstallIn(ServiceComponent::class)
abstract class ServiceModule {

    @Binds
    @ServiceScoped
    abstract fun provideNotificationService(wireGuardNotification: WireGuardNotification) : NotificationService

    @Binds
    @ServiceScoped
    abstract fun provideWifiService(wifiService: WifiService) : NetworkService<WifiService>

    @Binds
    @ServiceScoped
    abstract fun provideMobileDataService(mobileDataService : MobileDataService) : NetworkService<MobileDataService>
}
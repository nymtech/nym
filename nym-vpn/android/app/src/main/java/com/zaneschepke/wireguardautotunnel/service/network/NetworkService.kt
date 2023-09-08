package net.nymtech.nymconnect.service.network

import android.net.NetworkCapabilities
import kotlinx.coroutines.flow.Flow

interface NetworkService<T> {
    fun getNetworkName(networkCapabilities: NetworkCapabilities) : String?
    val networkStatus : Flow<NetworkStatus>

}
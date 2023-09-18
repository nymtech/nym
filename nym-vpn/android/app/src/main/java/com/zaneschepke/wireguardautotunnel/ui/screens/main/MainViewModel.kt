package net.nymtech.nymconnect.ui.screens.main

import android.annotation.SuppressLint
import android.app.Application
import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.wireguard.config.BadConfigException
import com.wireguard.config.Config
import net.nymtech.nymconnect.R
import net.nymtech.nymconnect.repository.Repository
import net.nymtech.nymconnect.service.foreground.ServiceState
import net.nymtech.nymconnect.service.foreground.ServiceManager
import net.nymtech.nymconnect.service.foreground.WireGuardConnectivityWatcherService
import net.nymtech.nymconnect.service.shortcut.ShortcutsManager
import net.nymtech.nymconnect.service.tunnel.VpnService
import net.nymtech.nymconnect.service.tunnel.model.Settings
import net.nymtech.nymconnect.service.tunnel.model.TunnelConfig
import net.nymtech.nymconnect.ui.ViewState
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import timber.log.Timber
import javax.inject.Inject


@HiltViewModel
class MainViewModel @Inject constructor(private val application : Application,
                                        private val tunnelRepo : Repository<TunnelConfig>,
                                        private val settingsRepo : Repository<Settings>,
                                        private val vpnService: VpnService,
) : ViewModel() {

    private val _viewState = MutableStateFlow(ViewState())
    val viewState get() = _viewState.asStateFlow()
    val tunnels get() = tunnelRepo.itemFlow
    val state get() = vpnService.state

    val handshakeStatus get() = vpnService.handshakeStatus
    val tunnelName get() = vpnService.tunnelName
    private val _settings = MutableStateFlow(Settings())
    val settings get() = _settings.asStateFlow()

    private val defaultConfigName = {
        "tunnel${(Math.random() * 100000).toInt()}"
    }


    init {
        viewModelScope.launch {
            settingsRepo.itemFlow.collect {
                val settings = it.first()
                validateWatcherServiceState(settings)
                _settings.emit(settings)
            }
        }
    }

    private fun validateWatcherServiceState(settings: Settings) {
        val watcherState = ServiceManager.getServiceState(application.applicationContext, WireGuardConnectivityWatcherService::class.java)
        if(settings.isAutoTunnelEnabled && watcherState == ServiceState.STOPPED && settings.defaultTunnel != null) {
            ServiceManager.startWatcherService(application.applicationContext, settings.defaultTunnel!!)
        }
    }


    fun onDelete(tunnel : TunnelConfig) {
        viewModelScope.launch {
            if(tunnelRepo.count() == 1L) {
                ServiceManager.stopWatcherService(application.applicationContext)
                val settings = settingsRepo.getAll()
                if(!settings.isNullOrEmpty()) {
                    val setting = settings[0]
                    setting.defaultTunnel = null
                    setting.isAutoTunnelEnabled = false
                    setting.isAlwaysOnVpnEnabled = false
                    settingsRepo.save(setting)
                }
            }
            tunnelRepo.delete(tunnel)
            ShortcutsManager.removeTunnelShortcuts(application.applicationContext, tunnel)
        }
    }

    fun onTunnelStart(tunnelConfig : TunnelConfig) = viewModelScope.launch {
        ServiceManager.startVpnService(application.applicationContext, tunnelConfig.toString())
    }

    fun onTunnelStop() {
        ServiceManager.stopVpnService(application.applicationContext)
    }

    fun onTunnelFileSelected(uri : Uri) {
        try {
            val fileName = getFileName(application.applicationContext, uri)
            val extension = getFileExtensionFromFileName(fileName)
            if(extension != ".conf") {
                viewModelScope.launch {
                    showSnackBarMessage(application.resources.getString(R.string.file_extension_message))
                }
                return
            }
            val stream = application.applicationContext.contentResolver.openInputStream(uri)
            stream ?: return
            val bufferReader = stream.bufferedReader(charset = Charsets.UTF_8)
                val config = Config.parse(bufferReader)
                val tunnelName = getNameFromFileName(fileName)
                saveTunnel(TunnelConfig(name = tunnelName, wgQuick = config.toWgQuickString()))
            stream.close()
        } catch(_: BadConfigException) {
            viewModelScope.launch {
                showSnackBarMessage(application.applicationContext.getString(R.string.bad_config))
            }
        }
    }

    private fun saveTunnel(tunnelConfig : TunnelConfig) {
        viewModelScope.launch {
            tunnelRepo.save(tunnelConfig)
            ShortcutsManager.createTunnelShortcuts(application.applicationContext, tunnelConfig)
        }
    }

    @SuppressLint("Range")
    private fun getFileName(context: Context, uri: Uri): String {
        if (uri.scheme == "content") {
            val cursor = try {
                context.contentResolver.query(uri, null, null, null, null)
            } catch (e : Exception) {
                Timber.d("Exception getting config name")
                null
            }
            cursor ?: return defaultConfigName()
            cursor.use {
                if(cursor.moveToFirst()) {
                    return cursor.getString(cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME))
                }
            }
        }
        return defaultConfigName()
    }

    suspend fun showSnackBarMessage(message : String) {
        _viewState.emit(_viewState.value.copy(
            showSnackbarMessage = true,
            snackbarMessage = message,
            snackbarActionText = "Okay",
            onSnackbarActionClick = {
                viewModelScope.launch {
                    dismissSnackBar()
                }
            }
        ))
        delay(3000)
        dismissSnackBar()
    }

    private suspend fun dismissSnackBar() {
        _viewState.emit(_viewState.value.copy(
            showSnackbarMessage = false
        ))
    }

    private fun getNameFromFileName(fileName : String) : String {
        return fileName.substring(0 , fileName.lastIndexOf('.') )
    }

    private fun getFileExtensionFromFileName(fileName : String) : String {
        return try {
            fileName.substring(fileName.lastIndexOf('.'))
        } catch (e : Exception) {
            ""
        }
    }
}
package net.nymtech.nyms5

import android.util.Log
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.work.Constraints
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequest
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.OutOfQuotaPolicy
import androidx.work.WorkInfo
import androidx.work.WorkManager
import kotlinx.coroutines.DelicateCoroutinesApi
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.GlobalScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

@OptIn(DelicateCoroutinesApi::class)
class MainViewModel(
    private val workManager: WorkManager,
    private val nymProxy: NymProxy
) : ViewModel() {
    private val tag = "viewModel"

    private val workRequest: OneTimeWorkRequest =
        OneTimeWorkRequestBuilder<ProxyWorker>()
            .setConstraints(
                Constraints.Builder()
                    .setRequiredNetworkType(NetworkType.CONNECTED).build()
            )
            .setExpedited(OutOfQuotaPolicy.RUN_AS_NON_EXPEDITED_WORK_REQUEST)
            .addTag(ProxyWorker.workTag)
            .setId(ProxyWorker.workId)
            .build()

    init {
        Log.d(tag, "____init")

        // When the work is cancelled "externally" ie. when the user tap the
        // "Stop" action on the notification, the underlying proxy process
        // keeps running in background
        // We have to manually call `stopClient` to kill it
        workManager.getWorkInfoByIdLiveData(ProxyWorker.workId)
            // watch "forever", ie. even when this viewModel has been cleared
            .observeForever { workInfo ->
                if (workInfo?.state == WorkInfo.State.CANCELLED || workInfo?.state == WorkInfo.State.FAILED) {
                    // ⚠ here one could be tempted to call cancelProxyWork
                    // but it uses viewModelScope which is cancelled when
                    // this viewModel instance is cleared
                    // use GlobalScope instead
                    GlobalScope.launch(Dispatchers.IO) {
                        // if the proxy process is still running ie. connected
                        // kill it
                        if (nymProxy.getState() == NymProxy.Companion.State.CONNECTED) {
                            Log.d(tag, "stopping proxy")
                            nymProxy.stop()
                            Log.i(tag, "proxy work cancelled")
                        }
                    }
                    setDisconnected()
                }
            }
    }

    data class ProxyState(
        val connected: Boolean = false,
        val loading: Boolean = false
    )

    // Expose screen UI state
    private val _uiState = MutableStateFlow(ProxyState())
    val uiState: StateFlow<ProxyState> = _uiState.asStateFlow()

    fun setConnected() {
        _uiState.update { currentState ->
            currentState.copy(
                connected = true,
                loading = false,
            )
        }
    }

    private fun setDisconnected() {
        _uiState.update { currentState ->
            currentState.copy(
                connected = false,
                loading = false,
            )
        }
    }

    fun startProxyWork() {
        // start loading state
        _uiState.update { currentState ->
            currentState.copy(
                connected = true,
                loading = true,
            )
        }

        // start long-running proxy service
        workManager.enqueueUniqueWork(
            ProxyWorker.name,
            ExistingWorkPolicy.REPLACE,
            workRequest
        )
    }

    fun cancelProxyWork() {
        // update state
        _uiState.update { currentState ->
            currentState.copy(
                connected = false,
                loading = true,
            )
        }
        viewModelScope.launch(Dispatchers.IO) {
            nymProxy.stop()
            // TODO instead of delaying an arbitrary amount of time here,
            //  rely on lib callback for the shutdown connection state
            // wait a bit to be sure the proxy client has enough time to
            // close connection
            delay(2000)
            setDisconnected()
        }
    }
}

class MainViewModelFactory(
    private val workManager: WorkManager,
    private val nymProxy: NymProxy
) :
    ViewModelProvider.Factory {
    override fun <T : ViewModel> create(modelClass: Class<T>): T {
        return if (modelClass.isAssignableFrom(MainViewModel::class.java)) {
            MainViewModel(workManager, nymProxy) as T
        } else {
            throw IllegalArgumentException("Unknown ViewModel class")
        }
    }
}
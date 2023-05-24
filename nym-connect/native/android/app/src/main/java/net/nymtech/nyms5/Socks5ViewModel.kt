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
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import net.nymtech.nyms5.ProxyWorker.Companion.State
import java.util.UUID

class Socks5ViewModel(
    private val workManager: WorkManager
) : ViewModel() {
    private val tag = "viewModel"

    private val workTag = "nymProxy"

    private val workId: UUID = UUID.randomUUID()

    private val workRequest: OneTimeWorkRequest = OneTimeWorkRequestBuilder<ProxyWorker>()
        .setConstraints(
            Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build()
        )
        .setExpedited(OutOfQuotaPolicy.RUN_AS_NON_EXPEDITED_WORK_REQUEST)
        .addTag(workTag)
        .setId(workId)
        .build()

    init {
        // observe the proxy work state
        workManager.getWorkInfoByIdLiveData(workId)
            .observeForever { workInfo ->
                if (workInfo?.state == WorkInfo.State.CANCELLED || workInfo?.state == WorkInfo.State.FAILED) {
                    // when the work is cancelled, ie. from the work notification "Stop" action
                    _uiState.update { currentState ->
                        currentState.copy(
                            loading = true,
                        )
                    }
                    stopProxy()
                    Log.d(tag, "proxy work cancelled")
                }
                if (workInfo != null && workInfo.state == WorkInfo.State.RUNNING) {
                    val progress = workInfo.progress.getString(State)
                    Log.d(tag, "work connection state $progress")
                    when (progress) {
                        "CONNECTED" -> if (!_uiState.value.connected || _uiState.value.loading) {
                            _uiState.update { currentState ->
                                currentState.copy(
                                    connected = true,
                                    loading = false,
                                )
                            }
                            Log.i(tag, "Nym proxy connected")
                        }

                        else -> {}
                    }
                }
            }
    }

    private val callback = object {
        fun onStop() {
            Log.d(tag, "⚡ ON STOP callback")
            _uiState.update { currentState ->
                currentState.copy(
                    connected = false,
                    loading = false,
                )
            }
            Log.i(tag, "Nym proxy disconnected")
        }
    }

    private val socks5 = Socks5()

    data class Socks5State(val connected: Boolean = false, val loading: Boolean = false)

    // Expose screen UI state
    private val _uiState = MutableStateFlow(Socks5State())
    val uiState: StateFlow<Socks5State> = _uiState.asStateFlow()

    private fun stopProxy() {
        viewModelScope.launch(Dispatchers.IO) {
            socks5.stop(callback)
        }
    }

    fun startProxyWork() {
        // start loading state
        _uiState.update { currentState ->
            currentState.copy(
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
                loading = true,
            )
        }
        stopProxy()
    }
}

class Socks5ViewModelFactory(private val workManager: WorkManager) : ViewModelProvider.Factory {
    override fun <T : ViewModel> create(modelClass: Class<T>): T {
        return if (modelClass.isAssignableFrom(Socks5ViewModel::class.java)) {
            Socks5ViewModel(workManager) as T
        } else {
            throw IllegalArgumentException("Unknown ViewModel class")
        }
    }
}
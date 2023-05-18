package net.nymtech.nyms5

import android.util.Log
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import androidx.work.Constraints
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.OutOfQuotaPolicy
import androidx.work.WorkManager
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class Socks5State(val connected: Boolean = false)

class Socks5ViewModel(
    private val workManager: WorkManager
) : ViewModel() {
    private val tag = "viewModel"

    private val workerTag = "nymProxy"

    private val socks5 = Socks5()

    // Expose screen UI state
    private val _uiState = MutableStateFlow(Socks5State())
    val uiState: StateFlow<Socks5State> = _uiState.asStateFlow()

    fun startSocks5() {
        val request = OneTimeWorkRequestBuilder<ProxyWorker>()
            .setConstraints(
                Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build()
            )
            .setExpedited(OutOfQuotaPolicy.RUN_AS_NON_EXPEDITED_WORK_REQUEST)
            .addTag(workerTag)
            .build()
        workManager.enqueue(request)

        _uiState.update { currentState ->
            currentState.copy(
                connected = true,
            )
        }
        Log.i(tag, "Nym socks5 started")
    }

    fun stopSocks5() {
        workManager.cancelAllWorkByTag(workerTag)
        viewModelScope.launch(Dispatchers.IO) {
            socks5.stop()
        }

        _uiState.update { currentState ->
            currentState.copy(
                connected = false,
            )
        }
        Log.i(tag, "Nym socks5 stopped")
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
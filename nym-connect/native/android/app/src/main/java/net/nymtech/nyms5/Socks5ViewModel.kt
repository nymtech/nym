package net.nymtech.nyms5

import android.util.Log
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

data class Socks5State(val connected: Boolean = false)

class Socks5ViewModel : ViewModel() {

    private val socks5 = Socks5()

    // Expose screen UI state
    private val _uiState = MutableStateFlow(Socks5State())
    val uiState: StateFlow<Socks5State> = _uiState.asStateFlow()

    fun startSocks5() {
        _uiState.update { currentState ->
            currentState.copy(
                connected = true,
            )
        }
        viewModelScope.launch(Dispatchers.IO) {
            socks5.start()
        }
        Log.i("App", "Nym socks5 started")
    }

    fun stopSocks5() {
        _uiState.update { currentState ->
            currentState.copy(
                connected = false,
            )
        }
        viewModelScope.launch(Dispatchers.IO) {
            socks5.stop()
        }
        Log.i("App", "Nym socks5 stopped")
    }
}

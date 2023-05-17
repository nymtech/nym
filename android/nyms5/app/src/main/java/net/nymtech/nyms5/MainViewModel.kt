package net.nymtech.nyms5

import android.util.Log
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

class MainViewModel : ViewModel() {
    init {
        viewModelScope.launch(Dispatchers.IO) {
            val result = Socks5().runClient()
            result.let { Log.d("App", "result: $it") }
        }
        Log.d("App", "libnyms5 CALLED")
    }

    // Expose screen UI state
    private val _uiState = MutableStateFlow(false)
    val uiState: StateFlow<Boolean> = _uiState.asStateFlow()

}

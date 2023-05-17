package net.nymtech.nyms5

import android.os.Bundle
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import kotlinx.coroutines.launch
import net.nymtech.nyms5.ui.theme.Nyms5Theme
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import kotlinx.coroutines.Dispatchers

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // TODO leave it for now, for testing purpose
        // it launches socks5 client calling `run` method
        /*val viewModel: MainViewModel by viewModels()
        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                viewModel.uiState.collect {
                    // Update UI elements
                }
            }
        }*/

        setContent {
            Nyms5Theme {
                // A surface container using the 'background' color from the theme
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    var connected by remember { mutableStateOf(false) }
                    Socks5Client(connected, {
                        lifecycleScope.launch(Dispatchers.IO) {
                            Socks5().runClient()
                        }
                        Log.i("App", "Nym Socks5 client started")
                        connected = true
                    }, {
                        lifecycleScope.launch(Dispatchers.IO) {
                            Socks5().stop()
                        }
                        Log.i("App", "Nym Socks5 client stopped")
                        connected = false
                    })
                }
            }
        }
    }
}

@Composable
fun Socks5Client(
    connected: Boolean,
    startClient: () -> Unit,
    stopClient: () -> Unit,
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier.padding(16.dp)) {
        Row(modifier = modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
            Text("socks5 client")
            Spacer(modifier = modifier.width(14.dp))
            Switch(checked = connected, onCheckedChange = {
                if (connected) stopClient() else startClient()
            })
        }
    }
}

@Preview
@Composable
fun PreviewSocks5Client() {
    var connected by remember { mutableStateOf(false) }
    Socks5Client(connected, {
        println("start socks5 client")
        connected = true
    }, {
        println("stop socks5 client")
        connected = false
    })
}

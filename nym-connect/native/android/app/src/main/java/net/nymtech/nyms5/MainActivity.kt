package net.nymtech.nyms5

import android.os.Bundle
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
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
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
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontStyle
import androidx.work.WorkManager

class MainActivity : ComponentActivity() {
    private val viewModel: Socks5ViewModel by viewModels {
        Socks5ViewModelFactory(
            workManager = WorkManager.getInstance(
                application.applicationContext
            )
        )
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                viewModel.uiState.collect {
                    setContent {
                        Nyms5Theme {
                            // A surface container using the 'background' color from the theme
                            Surface(
                                modifier = Modifier.fillMaxSize(),
                                color = MaterialTheme.colorScheme.background
                            ) {
                                S5ClientSwitch(it.connected, {
                                    when {
                                        it -> viewModel.startProxyWork()
                                        else -> viewModel.cancelProxyWork()
                                    }
                                })
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun S5ClientSwitch(
    connected: Boolean,
    onSwitch: (value: Boolean) -> Unit,
    modifier: Modifier = Modifier
) {
    val clipboardManager = LocalClipboardManager.current
    val proxyHost = stringResource(R.string.proxy_host)

    Column(modifier = modifier.padding(16.dp)) {
        Row(modifier = modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
            Text("Nym proxy")
            Spacer(modifier = modifier.width(14.dp))
            Switch(checked = connected, onCheckedChange = {
                onSwitch(!connected)
            })
        }
        if (connected) {
            Column(modifier = modifier.padding(16.dp)) {
                Text(
                    color = Color.Green,
                    fontStyle = FontStyle.Italic,
                    text = "Connected to the mixnet"
                )
                Row(
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(proxyHost)
                    Spacer(modifier = modifier.width(14.dp))
                    TextButton(onClick = {
                        clipboardManager.setText(AnnotatedString(proxyHost))
                    }) {
                        Text("Copy")
                    }
                }
            }
        }
    }
}

@Preview
@Composable
fun PreviewSocks5Client() {
    var connected by rememberSaveable { mutableStateOf(false) }
    S5ClientSwitch(connected, {
        when {
            it -> println("start socks5 client")
            else -> println("stop socks5 client")
        }
        connected = it
    })
}

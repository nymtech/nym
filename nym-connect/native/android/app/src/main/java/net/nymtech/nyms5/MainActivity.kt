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
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.LinearProgressIndicator
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
import net.nymtech.nyms5.ui.theme.NymTheme
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
    private val tag = "MainActivity"

    private val viewModel: Socks5ViewModel by viewModels {
        Socks5ViewModelFactory(
            workManager = WorkManager.getInstance(applicationContext),
            nymProxy = App.nymProxy
        )
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                viewModel.uiState.collect {
                    setContent {
                        NymTheme {
                            // A surface container using the 'background' color from the theme
                            Surface(
                                modifier = Modifier.fillMaxSize(),
                                color = MaterialTheme.colorScheme.background
                            ) {
                                val loading = it.loading

                                S5ClientSwitch(it.connected, loading, {
                                    if (!loading) {
                                        when {
                                            it -> {
                                                Log.d(tag, "switch ON")
                                                viewModel.startProxyWork()
                                            }
                                            else -> {
                                                Log.d(tag, "switch OFF")
                                                viewModel.cancelProxyWork()
                                            }
                                        }
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
    loading: Boolean,
    onSwitch: (value: Boolean) -> Unit,
    modifier: Modifier = Modifier
) {
    val clipboardManager = LocalClipboardManager.current
    val proxyHost = stringResource(R.string.proxy_host)
    if (loading) {
        Row {
            LinearProgressIndicator(
                modifier = modifier.fillMaxWidth(),
                color = MaterialTheme.colorScheme.secondary
            )
        }
    } else {
        Spacer(modifier = modifier.height(2.dp))
    }
    Column(modifier = modifier.padding(16.dp)) {
        Row(modifier = modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
            Text("Nym proxy")
            Spacer(modifier = modifier.width(14.dp))
            Switch(checked = connected, enabled = !loading, onCheckedChange = {
                onSwitch(!connected)
            })
        }
        if (connected && !loading) {
            Column(modifier = modifier.padding(16.dp)) {
                Text(
                    color = Color.Green,
                    fontStyle = FontStyle.Italic,
                    text = stringResource(R.string.connected_text)
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
    val tag = "UI"
    var connected by rememberSaveable { mutableStateOf(false) }
    var loading by rememberSaveable { mutableStateOf(false) }
    NymTheme {
        Surface(
            modifier = Modifier.fillMaxSize(),
            color = MaterialTheme.colorScheme.background
        ) {
            S5ClientSwitch(connected, loading, {
                when {
                    it -> Log.d(tag, "switch ON")
                    else -> Log.d(tag, "switch OFF")
                }
                connected = it
                loading = !loading
            })
        }
    }
}

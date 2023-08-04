package net.nymtech.nyms5.ui.composables

import android.util.Log
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import net.nymtech.nyms5.R
import net.nymtech.nyms5.ui.theme.NymTheme

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun S5ClientSwitch(
    connected: Boolean,
    loading: Boolean,
    onSwitch: (value: Boolean) -> Unit,
    modifier: Modifier = Modifier
) {
    val clipboardManager = LocalClipboardManager.current
    val host = stringResource(R.string.proxy_host)
    val port = stringResource(R.string.proxy_port)
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
        Row(
            modifier = modifier.padding(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text("Nym proxy")
            Spacer(modifier = modifier.width(14.dp))
            Switch(checked = connected, enabled = !loading, onCheckedChange = {
                onSwitch(!connected)
            }, modifier = Modifier.testTag("switch_connect"))
        }
        if (connected && !loading) {
            Column(modifier = modifier.padding(16.dp)) {
                Text(
                    color = Color.Green,
                    fontStyle = FontStyle.Italic,
                    text = stringResource(R.string.connected_text)
                )
                Spacer(modifier = modifier.height(10.dp))
                Row(
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Surface(
                        onClick = { clipboardManager.setText(AnnotatedString(host)) },
                        shape = RoundedCornerShape(6.dp),
                    ) {
                        Row(
                            modifier = modifier.padding(8.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text(host)
                            Spacer(modifier = modifier.width(8.dp))
                            Icon(
                                painter = painterResource(R.drawable.copy_24),
                                contentDescription = "copy to clipboard"
                            )
                        }
                    }
                    Spacer(modifier = modifier.width(12.dp))
                    Surface(
                        onClick = { clipboardManager.setText(AnnotatedString(port)) },
                        shape = RoundedCornerShape(6.dp),
                    ) {
                        Row(
                            modifier = modifier.padding(8.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text(port)
                            Spacer(modifier = modifier.width(8.dp))
                            Icon(
                                painter = painterResource(R.drawable.copy_24),
                                contentDescription = "copy to clipboard"
                            )
                        }
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
                loading = false
            })
        }
    }
}
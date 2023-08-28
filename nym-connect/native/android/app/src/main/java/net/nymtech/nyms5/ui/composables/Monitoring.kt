package net.nymtech.nyms5.ui.composables

import android.util.Log
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import net.nymtech.nyms5.R
import net.nymtech.nyms5.ui.theme.NymTheme
import net.nymtech.nyms5.ui.theme.darkYellow
import net.nymtech.nyms5.ui.theme.lightYellow

@Composable
fun Monitoring(
    modifier: Modifier = Modifier,
    initialValue: Boolean,
    onSwitch: (value: Boolean) -> Unit,
) {
    var monitoring by remember { mutableStateOf(initialValue) }
    val yellowColor = when (isSystemInDarkTheme()) {
        true -> darkYellow
        false -> lightYellow
    }

    Column(
        modifier = modifier
            .padding(16.dp)
            .verticalScroll(rememberScrollState())
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text("Enable error reporting")
            Spacer(modifier = modifier.width(16.dp))
            Switch(checked = monitoring, onCheckedChange = {
                monitoring = it
                onSwitch(it)
            })
        }
        Spacer(modifier = modifier.height(18.dp))
        Row(verticalAlignment = Alignment.CenterVertically) {
            Icon(
                painter = painterResource(R.drawable.warning_24),
                contentDescription = "copy to clipboard",
                tint = yellowColor
            )
            Spacer(modifier = modifier.width(16.dp))
            Text(
                stringResource(R.string.monitoring_desc_2),
                color = yellowColor
            )
        }
        Spacer(modifier = modifier.height(18.dp))
        Text(stringResource(R.string.monitoring_desc_1))
    }
}

@Preview
@Composable
fun PreviewMonitoring() {
    NymTheme {
        Surface(
            modifier = Modifier.fillMaxSize(),
            color = MaterialTheme.colorScheme.background
        ) {
            Monitoring(initialValue = false) {
                Log.d("Monitoring", "switch $it")
            }
        }
    }
}
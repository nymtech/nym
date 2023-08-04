package net.nymtech.nyms5.ui.composables

import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material3.CenterAlignedTopAppBar
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import net.nymtech.nyms5.MainViewModel
import net.nymtech.nyms5.R
import net.nymtech.nyms5.monitoringKey

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(
    proxyState: MainViewModel.ProxyState,
    monitoring: Boolean,
    dataStore: DataStore<Preferences>,
    onSwitch: (value: Boolean) -> Unit,
) {
    val navController = rememberNavController()
    var expanded by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    Scaffold(topBar = {
        CenterAlignedTopAppBar(
            title = {
                Text(stringResource(R.string.app_name))
            },
            navigationIcon = {
                val navBackStackEntry by navController.currentBackStackEntryAsState()
                val currentRoute = navBackStackEntry?.destination?.route

                if (currentRoute === "proxy") {
                    IconButton(onClick = { expanded = true }) {
                        Icon(
                            imageVector = Icons.Filled.Menu,
                            contentDescription = "Main menu"
                        )
                    }
                    DropdownMenu(
                        expanded = expanded,
                        onDismissRequest = { expanded = false }
                    ) {
                        DropdownMenuItem(onClick = {
                            navController.navigate("monitoring") {
                                popUpTo("proxy")
                            }
                            expanded = false
                        }, text = {
                            Text("Error reporting")
                        })
                    }
                } else {
                    IconButton(onClick = {
                        navController.navigate("proxy") {
                            popUpTo("proxy")
                        }
                    }) {
                        Icon(
                            imageVector = Icons.Filled.ArrowBack,
                            contentDescription = "Back home"
                        )
                    }
                }
            },
        )
    }) { contentPadding ->
        NavHost(
            navController = navController,
            startDestination = "proxy",
            modifier = Modifier.padding(contentPadding)
        ) {
            composable("proxy") {
                S5ClientSwitch(
                    connected = proxyState.connected,
                    loading = proxyState.loading,
                    onSwitch = onSwitch
                )
            }
            composable("monitoring") {
                Monitoring(initialValue = monitoring) {
                    scope.launch(Dispatchers.IO) {
                        dataStore.edit { settings -> settings[monitoringKey] = it }
                    }
                }
            }
        }
    }
}
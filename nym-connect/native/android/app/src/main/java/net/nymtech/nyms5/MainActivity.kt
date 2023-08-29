package net.nymtech.nyms5

import android.Manifest.permission.POST_NOTIFICATIONS
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.viewModels
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import kotlinx.coroutines.launch
import net.nymtech.nyms5.ui.theme.NymTheme
import androidx.work.WorkInfo
import androidx.work.WorkManager
import kotlinx.coroutines.DelicateCoroutinesApi
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.GlobalScope
import kotlinx.coroutines.flow.map
import net.nymtech.nyms5.ui.composables.HomeScreen

@OptIn(DelicateCoroutinesApi::class)
class MainActivity : ComponentActivity() {
    private val tag = "MainActivity"
    private val nymProxy = App.nymProxy

    private val viewModel: MainViewModel by viewModels {
        MainViewModelFactory(
            workManager = WorkManager.getInstance(applicationContext),
            nymProxy = App.nymProxy
        )
    }

    private val requestPermissionLauncher =
        registerForActivityResult(
            ActivityResultContracts.RequestPermission()
        ) { isGranted: Boolean ->
            if (isGranted) {
                Log.d(tag, "permission POST_NOTIFICATIONS has been granted")
            } else {
                Log.d(tag, "permission POST_NOTIFICATIONS has NOT been granted")
            }
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        Log.d(tag, "____onCreate")
        Log.i(tag, "device SDK [${Build.VERSION.SDK_INT}]")

        val workManager = WorkManager.getInstance(applicationContext)

        checkPermission()

        // observe proxy work progress
        workManager.getWorkInfoByIdLiveData(ProxyWorker.workId)
            // this observer is tied to the activity lifecycle
            .observe(this) { workInfo ->
                lifecycleScope.launch(Dispatchers.IO) {
                    val proxyState = App.nymProxy.getState()
                    val workState = workInfo.state
                    val workProgress = workInfo.progress.getString(ProxyWorker.State)
                    Log.i(
                        tag,
                        "proxy state: $proxyState, work state: $workState, work progress: $workProgress"
                    )
                    if (proxyState == NymProxy.Companion.State.CONNECTED &&
                        workState == WorkInfo.State.RUNNING
                    ) {
                        viewModel.setUiConnected()
                    }
                }
            }

        // The work can be cancelled by the user from the dedicated notification
        // by tapping the "Stop" action. When that happens the underlying proxy
        // process keeps running in the background
        // We have to manually call `stopClient` to kill it
        workManager.getWorkInfoByIdLiveData(ProxyWorker.workId)
            // watch "forever", ie. even when this viewModel has been cleared
            .observeForever { workInfo ->
                if (workInfo?.state == WorkInfo.State.CANCELLED || workInfo?.state == WorkInfo.State.FAILED) {
                    // ⚠ here one could be tempted to call `viewModel.cancelProxyWork`
                    // but it uses viewModelScope which is cancelled when
                    // the app goes to background so use `GlobalScope` instead
                    GlobalScope.launch(Dispatchers.IO) {
                        // if the proxy process is still running kill it
                        if (nymProxy.getState() == NymProxy.Companion.State.CONNECTED) {
                            Log.i(tag, "⚠ work has been cancelled")
                            nymProxy.stop()
                        }
                    }
                    // update the UI
                    viewModel.setUiDisconnected()
                }
            }

        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                Log.d(tag, "____UI recompose")
                applicationContext.dataStore.data.map { preferences ->
                    preferences[monitoringKey] ?: false
                }.collect { monitoring ->
                    viewModel.uiState.collect {
                        setContent {
                            NymTheme {
                                val loading = it.loading

                                HomeScreen(it, monitoring, applicationContext.dataStore) {
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
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    private fun checkPermission() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            // POST_NOTIFICATIONS is needed for the notification displayed
            // when socks5 proxy client is running
            when (checkSelfPermission(POST_NOTIFICATIONS)) {
                PackageManager.PERMISSION_GRANTED -> {
                    Log.d(tag, "check permission POST_NOTIFICATIONS: granted")
                }

                else -> {
                    Log.d(tag, "check permission POST_NOTIFICATIONS: not granted")
                    requestPermissionLauncher.launch(POST_NOTIFICATIONS)
                }
            }
        }
    }
}


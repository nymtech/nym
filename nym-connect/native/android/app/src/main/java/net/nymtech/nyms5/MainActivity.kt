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
import kotlinx.coroutines.flow.map
import net.nymtech.nyms5.ui.composables.HomeScreen

class MainActivity : ComponentActivity() {
    private val tag = "MainActivity"

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

        checkPermission()

        // observe proxy work progress
        WorkManager.getInstance(applicationContext)
            .getWorkInfoByIdLiveData(ProxyWorker.workId)
            // this observer is tied to the activity lifecycle
            .observe(this) { workInfo ->
                if (workInfo != null && workInfo.state == WorkInfo.State.RUNNING) {
                    val progress =
                        workInfo.progress.getString(ProxyWorker.State)
                    when (progress) {
                        ProxyWorker.Work.Status.CONNECTED.name -> {
                            Log.i(tag, "Nym proxy $progress")
                            viewModel.setConnected()
                        }

                        else -> Log.i(tag, "Nym proxy $progress")
                    }
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

    override fun onStart() {
        super.onStart()
        viewModel.checkStateSync()
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


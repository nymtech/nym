package net.nymtech.nymconnect.ui

import android.Manifest
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import android.view.KeyEvent
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.compose.animation.ExperimentalAnimationApi
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.slideInHorizontally
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.input.key.onKeyEvent
import com.google.accompanist.navigation.animation.AnimatedNavHost
import com.google.accompanist.navigation.animation.composable
import com.google.accompanist.navigation.animation.rememberAnimatedNavController
import com.google.accompanist.permissions.ExperimentalPermissionsApi
import com.google.accompanist.permissions.isGranted
import com.google.accompanist.permissions.rememberPermissionState
import com.wireguard.android.backend.GoBackend
import net.nymtech.nymconnect.R
import net.nymtech.nymconnect.ui.common.PermissionRequestFailedScreen
import net.nymtech.nymconnect.ui.common.navigation.BottomNavBar
import net.nymtech.nymconnect.ui.screens.config.ConfigScreen
import net.nymtech.nymconnect.ui.screens.detail.DetailScreen
import net.nymtech.nymconnect.ui.screens.main.MainScreen
import net.nymtech.nymconnect.ui.screens.settings.SettingsScreen
import net.nymtech.nymconnect.ui.screens.support.SupportScreen
import net.nymtech.nymconnect.ui.theme.TransparentSystemBars
import net.nymtech.nymconnect.ui.theme.WireguardAutoTunnelTheme
import dagger.hilt.android.AndroidEntryPoint
import timber.log.Timber
import java.lang.IllegalStateException

@AndroidEntryPoint
class MainActivity : AppCompatActivity() {

    @OptIn(ExperimentalAnimationApi::class,
        ExperimentalPermissionsApi::class
    )
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            val navController = rememberAnimatedNavController()
            val focusRequester = remember { FocusRequester() }

            WireguardAutoTunnelTheme {
                TransparentSystemBars()

                val snackbarHostState = remember { SnackbarHostState() }

                val notificationPermissionState =
                    rememberPermissionState(Manifest.permission.POST_NOTIFICATIONS)

                fun requestNotificationPermission() {
                    if (!notificationPermissionState.status.isGranted && Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                        notificationPermissionState.launchPermissionRequest()
                    }
                }

                var vpnIntent by remember { mutableStateOf(GoBackend.VpnService.prepare(this)) }
                val vpnActivityResultState = rememberLauncherForActivityResult(
                    ActivityResultContracts.StartActivityForResult(),
                    onResult = {
                        val accepted = (it.resultCode == RESULT_OK)
                        if (accepted) {
                            vpnIntent = null
                        }
                    })
                LaunchedEffect(vpnIntent) {
                    if (vpnIntent != null) {
                        vpnActivityResultState.launch(vpnIntent)
                    } else requestNotificationPermission()
                }

                Scaffold(snackbarHost = { SnackbarHost(snackbarHostState)},
                    modifier = Modifier.onKeyEvent {
                        if (it.nativeKeyEvent.action == KeyEvent.ACTION_UP) {
                            when (it.nativeKeyEvent.keyCode) {
                                KeyEvent.KEYCODE_DPAD_UP -> {
                                    try {
                                        focusRequester.requestFocus()
                                    } catch(e : IllegalStateException) {
                                        Timber.e("No D-Pad focus request modifier added to element on screen")
                                    }
                                    false
                                } else -> {
                                   false;
                                }
                            }
                        } else {
                            false
                        }
                    },
                    bottomBar = if (vpnIntent == null && notificationPermissionState.status.isGranted) {
                        { BottomNavBar(navController, Routes.navItems) }
                    } else {
                        {}
                    },
                )
                { padding ->
                    if (vpnIntent != null) {
                        PermissionRequestFailedScreen(
                            padding = padding,
                            onRequestAgain = { vpnActivityResultState.launch(vpnIntent) },
                            message = getString(R.string.vpn_permission_required),
                            getString(R.string.retry)
                        )
                        return@Scaffold
                    }
                    if (!notificationPermissionState.status.isGranted) {
                        PermissionRequestFailedScreen(
                            padding = padding,
                            onRequestAgain = {
                                val intentSettings =
                                    Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS)
                                intentSettings.data =
                                    Uri.fromParts("package", this.packageName, null)
                                startActivity(intentSettings);
                            },
                            message = getString(R.string.notification_permission_required),
                            getString(R.string.open_settings)
                        )
                        return@Scaffold
                    }
                    AnimatedNavHost(navController, startDestination = Routes.Main.name) {
                        composable(Routes.Main.name, enterTransition = {
                            when (initialState.destination.route) {
                                Routes.Settings.name, Routes.Support.name ->
                                    slideInHorizontally(
                                        initialOffsetX = { -1000 },
                                        animationSpec = tween(500)
                                    )

                                else -> {
                                    fadeIn(animationSpec = tween(1000))
                                }
                            }
                        }) {
                            MainScreen(padding = padding, snackbarHostState = snackbarHostState, navController = navController)
                        }
                        composable(Routes.Settings.name, enterTransition = {
                            when (initialState.destination.route) {
                                Routes.Main.name ->
                                    slideInHorizontally(
                                        initialOffsetX = { 1000 },
                                        animationSpec = tween(500)
                                    )

                                Routes.Support.name -> {
                                    slideInHorizontally(
                                        initialOffsetX = { -1000 },
                                        animationSpec = tween(500)
                                    )
                                }

                                else -> {
                                    fadeIn(animationSpec = tween(1000))
                                }
                            }
                        }) { SettingsScreen(padding = padding, snackbarHostState = snackbarHostState, navController = navController, focusRequester = focusRequester) }
                        composable(Routes.Support.name, enterTransition = {
                            when (initialState.destination.route) {
                                Routes.Settings.name, Routes.Main.name ->
                                    slideInHorizontally(
                                        initialOffsetX = { 1000 },
                                        animationSpec = tween(500)
                                    )

                                else -> {
                                    fadeIn(animationSpec = tween(1000))
                                }
                            }
                        }) { SupportScreen(padding = padding, focusRequester) }
                        composable("${Routes.Config.name}/{id}", enterTransition = {
                            fadeIn(animationSpec = tween(1000))
                        }) { ConfigScreen(padding = padding, navController = navController, id = it.arguments?.getString("id"), focusRequester = focusRequester)}
                        composable("${Routes.Detail.name}/{id}", enterTransition = {
                            fadeIn(animationSpec = tween(1000))
                        }) { DetailScreen(padding = padding, id = it.arguments?.getString("id")) }
                    }
                }
            }
        }
    }
}

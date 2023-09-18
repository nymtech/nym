package net.nymtech.nymconnect.ui

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Home
import androidx.compose.material.icons.rounded.QuestionMark
import androidx.compose.material.icons.rounded.Settings
import net.nymtech.nymconnect.ui.common.navigation.BottomNavItem

enum class Routes {
    Main,
    Settings,
    Support,
    Config,
    Detail;


    companion object {
        val navItems = listOf(
            BottomNavItem(
                name = "Tunnels",
                route = Main.name,
                icon = Icons.Rounded.Home,
            ),
            BottomNavItem(
                name = "Settings",
                route = Settings.name,
                icon = Icons.Rounded.Settings,
            ),
            BottomNavItem(
                name = "Support",
                route = Support.name,
                icon = Icons.Rounded.QuestionMark,
            )
        )
    }
}
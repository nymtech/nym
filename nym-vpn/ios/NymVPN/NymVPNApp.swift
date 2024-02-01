import SwiftUI
import Home
import Theme
import AppSettings
import Tunnels

@main
struct NymVPNApp: App {
    init() {
        setup()
    }

    var body: some Scene {
        WindowGroup {
            NavigationStack {
                HomeView()
            }
            .preferredColorScheme(AppSettings.shared.currentTheme.colorScheme)
            .environmentObject(AppSettings.shared)
            .environmentObject(TunnelsManager.shared)
        }
    }
}

private extension NymVPNApp {
    func setup() {
        ThemeConfiguration.setup()
    }
}

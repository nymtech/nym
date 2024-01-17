import SwiftUI
import Home
import Theme
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
            .environmentObject(TunnelsManager.shared)
        }
    }
}

private extension NymVPNApp {
    func setup() {
        ThemeConfiguration.setup()
    }
}

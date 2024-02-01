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
                HomeView(viewModel: HomeViewModel(selectedNetwork: .mixnet))
            }
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

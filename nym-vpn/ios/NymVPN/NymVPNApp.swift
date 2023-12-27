import SwiftUI
import Home
import Theme

@main
struct NymVPNApp: App {
    init() {
        ThemeConfiguration.setup()
    }

    var body: some Scene {
        WindowGroup {
            NavigationStack {
                HomeView()
            }
        }
    }
}

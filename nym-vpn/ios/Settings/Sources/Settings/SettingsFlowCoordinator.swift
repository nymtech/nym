import SwiftUI

struct SettingsFlowCoordinator<Content: View>: View {
    @ObservedObject var state: SettingsFlowState
    let content: () -> Content

    var body: some View {
        content()
            .navigationDestination(for: SettingsLink.self, destination: linkDestination)
    }

    @ViewBuilder private func linkDestination(link: SettingsLink) -> some View {
        EmptyView()
    }
}

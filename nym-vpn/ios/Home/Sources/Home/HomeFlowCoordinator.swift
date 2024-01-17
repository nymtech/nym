import SwiftUI
import Settings

struct HomeFlowCoordinator<Content: View>: View {
    @ObservedObject var state: HomeFlowState
    let content: () -> Content

    var body: some View {
        NavigationStack(path: $state.path) {
            ZStack {
                content()
            }
            .navigationDestination(for: HomeLink.self, destination: linkDestination)
        }
    }
}

private extension HomeFlowCoordinator {
    @ViewBuilder private func linkDestination(link: HomeLink) -> some View {
        switch link {
        case .firstHop(text: _):
            HopListView(viewModel: HopListViewModel(path: $state.path, type: .first))
        case .lastHop:
            HopListView(viewModel: HopListViewModel(path: $state.path, type: .last))
        case .settings:
            SettingsView(viewModel: SettingsViewModel(path: $state.path))
        }
    }
}

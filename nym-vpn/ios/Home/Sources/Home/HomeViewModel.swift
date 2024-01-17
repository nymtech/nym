import SwiftUI
import UIComponents
import Tunnels

public class HomeViewModel: HomeFlowState {
    private let tunnelsManager: TunnelsManager

    @Published var selectedNetwork: NetworkButtonViewModel.ButtonType

    public init(
        selectedNetwork: NetworkButtonViewModel.ButtonType,
        tunnelsManager: TunnelsManager = TunnelsManager.shared
    ) {
        self.selectedNetwork = selectedNetwork
        self.tunnelsManager = tunnelsManager

        tunnelsManager.loadConfigurations()
    }
}

// MARK: - Navigation -

public extension HomeViewModel {
    func navigateToSettings() {
        path.append(HomeLink.settings)
    }

    func navigateToFirstHopSelection() {
        path.append(HomeLink.firstHop(text: ""))
    }

    func navigateToLastHopSelection() {
        path.append(HomeLink.lastHop)
    }
}

// MARK: - Tunnel testing -

public extension HomeViewModel {
    func connect() {
        if let tunnel = tunnelsManager.currentTunnel, tunnel.tunnel.connection.status == .connected {
            tunnelsManager.disconnect()
        } else {
            tunnelsManager.test()
        }
    }
}

import SwiftUI
import UIComponents
import Theme

public struct HomeView: View {
    public init() {}

    public var body: some View {
        VStack {
            CustomNavBar(
                title: "NymVPN".localizedString,
                rightButtonConfig: settingsButtonConfig()
            )
            Spacer()
                .frame(height: 50)
            StatusButton(config: .connected)
            Spacer()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background {
            NymColor.background
                .ignoresSafeArea()
        }
    }
}

private extension HomeView {
    func settingsButtonConfig() -> CustomNavBarButtonConfig {
        CustomNavBarButtonConfig(type: .settingsGear) {}
    }
}

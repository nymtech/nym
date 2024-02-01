import SwiftUI
import AppSettings
import Modifiers
import UIComponents
import Theme

public struct SettingsView: View {
    @StateObject private var viewModel: SettingsViewModel

    public init(viewModel: SettingsViewModel) {
        _viewModel = StateObject(wrappedValue: viewModel)
    }

    public var body: some View {
        SettingsFlowCoordinator(state: viewModel, content: content)
    }
}

private extension SettingsView {
    @ViewBuilder
    func content() -> some View {
        VStack {
            navbar()
            settingsList()
            Spacer()
        }
        .appearanceUpdate()
        .navigationBarBackButtonHidden(true)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .ignoresSafeArea(edges: [.bottom])
        .background {
            NymColor.background
                .ignoresSafeArea()
        }
    }

    @ViewBuilder
    func navbar() -> some View {
        CustomNavBar(
            title: viewModel.settingsTitle,
            leftButton: CustomNavBarButton(type: .back, action: { viewModel.navigateHome() })
        )
    }

    @ViewBuilder
    func settingsList() -> some View {
        SettingsList(
            viewModel:
                SettingsListViewModel(
                    sections: viewModel.sections,
                    appVersion: viewModel.appVersion()
                )
        )
    }
}

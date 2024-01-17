import SwiftUI

public struct SettingsView: View {
    @StateObject var viewModel: SettingsViewModel

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
        Text(/*@START_MENU_TOKEN@*/"Hello, World!"/*@END_MENU_TOKEN@*/)
    }
}

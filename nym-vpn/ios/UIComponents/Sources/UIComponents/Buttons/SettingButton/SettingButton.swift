import SwiftUI
import Theme

public struct SettingButton: View {
    private let viewModel: SettingButtonViewModel

    public init(viewModel: SettingButtonViewModel) {
        self.viewModel = viewModel
    }

    public var body: some View {
        VStack {
            HStack {
                Image(viewModel.selectionImageName, bundle: .module)
                    .foregroundStyle(viewModel.selectionImageColor)
                    .padding(.leading, 16)

                VStack(alignment: .leading) {
                    Text(viewModel.title)
                        .foregroundStyle(NymColor.sysOnSurface)
                        .textStyle(.Body.Large.primary)
                    if let subtitle = viewModel.subtitle {
                        Text(subtitle)
                            .foregroundStyle(NymColor.sysOutline)
                            .textStyle(.Body.Medium.primary)
                    }
                }
                .padding(.leading, 8)
                Spacer()
            }
        }
        .frame(maxWidth: .infinity, minHeight: 64, maxHeight: 64)
        .background(NymColor.navigationBarBackground)
        .cornerRadius(8)
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .inset(by: 0.5)
                .stroke(viewModel.selectionStrokeColor)
        )
    }
}

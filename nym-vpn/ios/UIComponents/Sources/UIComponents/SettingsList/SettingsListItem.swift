import SwiftUI
import Theme

public struct SettingsListItem: View {
    private let viewModel: SettingsListItemViewModel

    public init(viewModel: SettingsListItemViewModel) {
        self.viewModel = viewModel
    }

    public var body: some View {
        VStack(spacing: 0) {
            Spacer()
            HStack {
                iconImage()
                    .padding(.leading, 16)

                titleSubtitle()
                Spacer()

                accessoryImage()
            }
            Spacer()
            optionalDivider()
        }
        .frame(maxWidth: .infinity, minHeight: 64, maxHeight: 64)
        .background(NymColor.navigationBarBackground)
        .clipShape(
            .rect(
                topLeadingRadius: viewModel.topRadius,
                bottomLeadingRadius: viewModel.bottomRadius,
                bottomTrailingRadius: viewModel.bottomRadius,
                topTrailingRadius: viewModel.topRadius
            )
        )
        .padding(.horizontal, 16)
        .onTapGesture {
            viewModel.action()
        }
    }
}

private extension SettingsListItem {
    @ViewBuilder
    func optionalDivider() -> some View {
        if !viewModel.position.isLast {
            Divider()
                .frame(height: 1)
                .overlay(NymColor.settingsSeparator)
        }
    }

    @ViewBuilder
    func iconImage() -> some View {
        if let imageName = viewModel.imageName {
            Image(imageName, bundle: .module)
                .foregroundStyle(NymColor.sysOnSurface)
                .padding(.leading, 8)
        }
    }

    @ViewBuilder
    func titleSubtitle() -> some View {
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
        .padding(.leading, 16)
    }

    @ViewBuilder
    func accessoryImage() -> some View {
        if let imageName = viewModel.accessory.imageName {
            Image(imageName, bundle: .module)
                .foregroundStyle(NymColor.sysOnSurface)
                .padding(.trailing, 24)
        }
    }
}

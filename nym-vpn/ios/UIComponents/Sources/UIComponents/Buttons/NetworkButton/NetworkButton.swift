import SwiftUI
import Theme

public struct NetworkButton: View {
    private let viewModel: NetworkButtonViewModel

    public init(viewModel: NetworkButtonViewModel) {
        self.viewModel = viewModel
    }

    public var body: some View {
        VStack {
            HStack {
                Image(viewModel.selectionImageName, bundle: .module)
                    .foregroundStyle(viewModel.selectionImageColor)
                    .padding(.leading, 16)

                Image(viewModel.type.imageName, bundle: .module)
                    .foregroundStyle(NymColor.sysOnSurface)
                    .padding(.leading, 8)

                VStack(alignment: .leading) {
                    Text(viewModel.type.title)
                        .foregroundStyle(NymColor.sysOnSurface)
                        .textStyle(.Body.Large.primary)
                    Text(viewModel.type.subtitle)
                        .foregroundStyle(NymColor.sysOutline)
                        .textStyle(.Body.Medium.primary)
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

public struct NetworkButtonViewModel {
    public enum ButtonType {
        case mixnet
        case wireguard

        var imageName: String {
            switch self {
            case .mixnet:
                return "mixnetIcon"
            case .wireguard:
                return "wireguardIcon"
            }
        }

        var title: String {
            switch self {
            case .mixnet:
                "5hopMixnetTitle".localizedString
            case .wireguard:
                "2hopWireGuardTitle".localizedString
            }
        }

        var subtitle: String {
            switch self {
            case .mixnet:
                "5hopMixnetSubtitle".localizedString
            case .wireguard:
                "2hopWireGuardSubtitle".localizedString
            }
        }
    }

    let type: ButtonType

    @Binding var selectedNetwork: ButtonType

    public init(type: ButtonType, selectedNetwork: Binding<ButtonType>) {
        self.type = type
        self._selectedNetwork = selectedNetwork
    }

    private var isSelected: Bool {
        type == selectedNetwork
    }

    var selectionImageName: String {
        isSelected ? "networkSelectedCircle" : "networkCircle"
    }

    var selectionImageColor: Color {
        isSelected ? NymColor.primaryOrange : NymColor.networkButtonCircle
    }

    var selectionStrokeColor: Color {
        isSelected ? NymColor.primaryOrange : .clear
    }
}

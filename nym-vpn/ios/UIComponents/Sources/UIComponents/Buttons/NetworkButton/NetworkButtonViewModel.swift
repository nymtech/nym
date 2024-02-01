import SwiftUI
import Theme

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

import SwiftUI
import Theme

public enum StatusButtonConfig: String {
    case connected
    case connecting
    case disconnecting
    case disconnected
    case error

    var title: String {
        self.rawValue.localizedString
    }

    var textColor: Color {
        switch self {
        case .connected:
            return NymColor.confirm
        case .connecting, .disconnecting:
            return NymColor.statusButtonTitleConnecting
        case .disconnected ,.error:
            return NymColor.sysOnSecondary
        }
    }

    var backgroundColor: Color {
        switch self {
        case .connected:
            return NymColor.statusGreen
        case .connecting, .disconnecting, .disconnected, .error:
            return NymColor.statusButtonBackground
        }
    }
}

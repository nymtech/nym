import SwiftUI
import Theme

public struct CustomNavBarButton: View {
    public enum ButtonType: String {
        case back
        case settings
        case empty

        var imageName: String? {
            switch self {
            case .back:
                "arrowBack"
            case .settings:
                "settingsGear"
            case .empty:
                nil
            }
        }
    }

    public let type: ButtonType
    public let action: (() -> Void)?

    public init(type: ButtonType, action: (() -> Void)?) {
        self.type = type
        self.action = action
    }

    public var body: some View {
        Button {
            action?()
        } label: {
            if let imageName = type.imageName {
                Image(imageName, bundle: .module)
                    .foregroundStyle(NymColor.navigationBarSettingsGear)
            }
        }
        .frame(width: 48, height: 48)
    }
}

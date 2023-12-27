public struct CustomNavBarButtonConfig {
    public enum ButtonType: String {
        case settingsGear

        var imageName: String {
            self.rawValue
        }
    }

    public init(type: ButtonType? = nil, action: (() -> Void)?) {
        self.type = type
        self.action = action
    }

    public let type: ButtonType?
    public let action: (() -> Void)?
}

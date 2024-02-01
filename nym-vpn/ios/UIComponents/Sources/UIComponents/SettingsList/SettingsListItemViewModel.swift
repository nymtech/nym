import Foundation

public final class SettingsListItemViewModel: Hashable {
    public enum Accessory: String {
        case toggle
        case arrow
        case empty

        var imageName: String? {
            switch self {
            case .arrow:
                return "arrowRight"
            default:
                return nil
            }
        }
    }

    let title: String
    let subtitle: String?
    let imageName: String?
    let accessory: Accessory
    let action: (() -> Void)

    var position: SettingsListItemPosition

    public init(
        accessory: Accessory,
        title: String,
        subtitle: String? = nil,
        imageName: String? = nil,
        position: SettingsListItemPosition = SettingsListItemPosition(isFirst: false, isLast: false),
        action: @escaping (() -> Void)
    ) {
        self.title = title
        self.subtitle = subtitle
        self.imageName = imageName
        self.accessory = accessory
        self.position = position
        self.action = action
    }

    var topRadius: CGFloat {
        if position.isFirst {
            return CGFloat(8)
        } else {
            return CGFloat(0)
        }
    }

    var bottomRadius: CGFloat {
        if position.isLast {
            return CGFloat(8)
        } else {
            return CGFloat(0)
        }
    }

    public func hash(into hasher: inout Hasher) {
        hasher.combine(title)
        hasher.combine(subtitle)
        hasher.combine(imageName)
        hasher.combine(accessory)
    }

    public static func == (lhs: SettingsListItemViewModel, rhs: SettingsListItemViewModel) -> Bool {
        lhs.hashValue == rhs.hashValue
    }
}

public struct SettingsListItemPosition: Hashable {
    public var isFirst: Bool
    public var isLast: Bool

    public init(isFirst: Bool, isLast: Bool) {
        self.isFirst = isFirst
        self.isLast = isLast
    }
}

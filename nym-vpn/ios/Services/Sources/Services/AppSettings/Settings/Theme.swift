import SwiftUI

public enum Theme: Int {
    case light
    case dark
    case automatic

    public var colorScheme: ColorScheme? {
        switch self {
        case .light:
            return .light
        case .dark:
            return .dark
        case .automatic:
            return ColorScheme(.unspecified)
        }
    }
}

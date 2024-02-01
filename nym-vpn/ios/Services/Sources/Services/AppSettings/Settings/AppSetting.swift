import SwiftUI

public struct AppSetting {
    public enum Appearance: Int, CaseIterable {
        case automatic
        case light
        case dark

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
}

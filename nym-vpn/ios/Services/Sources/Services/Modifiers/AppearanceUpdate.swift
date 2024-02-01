import SwiftUI
import AppSettings

public struct AppearanceUpdate: ViewModifier {
    @EnvironmentObject private var appSettings: AppSettings

    public func body(content: Content) -> some View {
        content
            .preferredColorScheme(appSettings.currentTheme.colorScheme)
    }
}

public extension View {
    func appearanceUpdate() -> some View {
        modifier(AppearanceUpdate())
    }
}

import SwiftUI
import AppSettings
import Theme

public struct AppearanceViewModel {
    private let appSettings: AppSettings

    let title = "displayTheme".localizedString

    @Binding var path: NavigationPath

    var themes: [AppSetting.Appearance] {
        AppSetting.Appearance.allCases
    }

    var currentTheme: AppSetting.Appearance {
        appSettings.currentTheme
    }

    public init(path: Binding<NavigationPath>, appSettings: AppSettings) {
        _path = path
        self.appSettings = appSettings
    }

    func setCurrentTheme(with theme: AppSetting.Appearance) {
        appSettings.currentTheme = theme
    }
}

extension AppearanceViewModel {
    func themeTitle(for theme: AppSetting.Appearance) -> String {
        switch theme {
        case .light:
            return "lightThemeTitle".localizedString
        case .dark:
            return "darkThemeTitle".localizedString
        case .automatic:
            return "automaticThemeTitle".localizedString
        }
    }

    func themeSubtitle(for theme: AppSetting.Appearance) -> String? {
        switch theme {
        case .light, .dark:
            return nil
        case .automatic:
            return "automaticThemeSubtitle".localizedString
        }
    }

    func isSelected(for theme: AppSetting.Appearance) -> Bool {
        appSettings.currentTheme == theme
    }
}

extension AppearanceViewModel {
    func navigateBack() {
        path.removeLast()
    }
}

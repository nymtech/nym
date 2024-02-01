import Foundation
import AppVersionProvider

public class SettingsViewModel: SettingsFlowState {
    let settingsTitle = "settings".localizedString
    let settingsConfig = SettingsConfig()

    func navigateHome() {
        path = .init()
    }

    func appVersion() -> String {
        AppVersionProvider.appVersion()
    }
}

private extension SettingsViewModel {
    func navigateToTheme() {
        path.append(SettingsLink.theme)
    }
}

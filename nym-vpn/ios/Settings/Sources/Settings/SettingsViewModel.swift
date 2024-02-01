import Foundation
import AppVersionProvider
import UIComponents

public class SettingsViewModel: SettingsFlowState {
    let settingsTitle = "settings".localizedString

    var sections: [SettingsSection] {
        [
            connectionSection(),
            themeSection(),
            logsSection(),
            feedbackSection(),
            legalSection()
        ]
    }

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

private extension SettingsViewModel {
    func connectionSection() -> SettingsSection {
        .connection(
            viewModels: [
                SettingsListItemViewModel(
                    accessory: .arrow,
                    title: "autoConnectTitle".localizedString,
                    subtitle: "autoConnectSubtitle".localizedString,
                    imageName: "autoConnect",
                    action: {}
                ),
                SettingsListItemViewModel(
                    accessory: .toggle,
                    title: "entryLocationTitle".localizedString,
                    subtitle: "entryLocationSubtitle".localizedString,
                    imageName: "entryHop",
                    action: {}
                )
            ]
        )
    }

    func themeSection() -> SettingsSection {
        .theme(
            viewModels: [
                SettingsListItemViewModel(
                    accessory: .arrow,
                    title: "displayTheme".localizedString,
                    imageName: "displayTheme",
                    action: { [weak self] in
                        self?.navigateToTheme()
                    }
                )
            ]
        )
    }

    func logsSection() -> SettingsSection {
        .logs(
            viewModels: [
                SettingsListItemViewModel(
                    accessory: .arrow,
                    title: "logs".localizedString,
                    imageName: "logs",
                    action: {}
                )
            ]
        )
    }

    func feedbackSection() -> SettingsSection {
        .feedback(
            viewModels: [
                SettingsListItemViewModel(
                    accessory: .arrow,
                    title: "feedback".localizedString,
                    imageName: "feedback",
                    action: {}
                ),
                SettingsListItemViewModel(
                    accessory: .arrow,
                    title: "support".localizedString,
                    imageName: "support",
                    action: {}
                )
            ]
        )
    }

    func legalSection() -> SettingsSection {
        .legal(
            viewModels: [
                SettingsListItemViewModel(
                    accessory: .arrow,
                    title: "legal".localizedString,
                    action: {}
                )
            ]
        )
    }
}

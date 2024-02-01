import Theme
import UIComponents

public struct SettingsConfig {
    public init() {}

    var sections: [SettingsSection] {
        [
            connectionSection(),
            themeSection(),
            logsSection(),
            feedbackSection(),
            legalSection()
        ]
    }
}

private extension SettingsConfig {
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
                    action: {

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

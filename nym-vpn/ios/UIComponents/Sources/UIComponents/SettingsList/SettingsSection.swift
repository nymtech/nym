import Foundation

public enum SettingsSection: Hashable {
    case connection(viewModels: [SettingsListItemViewModel])
    case theme(viewModels: [SettingsListItemViewModel])
    case logs(viewModels: [SettingsListItemViewModel])
    case feedback(viewModels: [SettingsListItemViewModel])
    case legal(viewModels: [SettingsListItemViewModel])

    var settingsViewModels: [SettingsListItemViewModel] {
        switch self {
        case let .connection(viewModels),
            let .theme(viewModels),
            let .logs(viewModels),
            let .feedback(viewModels),
            let .legal(viewModels):
            return viewModels
        }
    }
}

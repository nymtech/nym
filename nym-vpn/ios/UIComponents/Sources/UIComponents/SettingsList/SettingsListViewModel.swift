import Foundation
import Theme

public struct SettingsListViewModel {
    private let appVersion: String

    let sections: [SettingsSection]

    var versionTitle: String {
        "version".localizedString + " \(appVersion)"
    }

    public init(sections: [SettingsSection], appVersion: String) {
        self.sections = sections
        self.appVersion = appVersion
    }
}

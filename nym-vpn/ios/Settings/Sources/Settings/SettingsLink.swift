import Foundation

enum SettingsLink: Hashable, Identifiable {
    case theme

    var id: String {
        String(describing: self)
    }
}

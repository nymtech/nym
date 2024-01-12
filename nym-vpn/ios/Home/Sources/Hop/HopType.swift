import Foundation

public enum HopType {
    case first
    case last

    var selectHopLocalizedTitle: String {
        switch self {
        case .first:
            "firstHopSelection".localizedString
        case .last:
            "lastHopSelection".localizedString
        }
    }
}

import Foundation

public extension String {
    var localizedString: String {
        Bundle.module.localizedString(forKey: self)
    }
}

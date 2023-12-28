import Foundation

extension Bundle {
    func localizedString(forKey key: String) -> String {
        localizedString(forKey: key, value: nil, table: nil)
    }
}

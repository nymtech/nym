import Foundation

enum HomeLink: Hashable, Identifiable {
    case firstHop(text: String?)
    case lastHop
    case settings

    var id: String {
        String(describing: self)
    }
}

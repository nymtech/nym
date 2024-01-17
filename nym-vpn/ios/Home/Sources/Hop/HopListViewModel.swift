import SwiftUI

public struct HopListViewModel {
    public let type: HopType

    @Binding var path: NavigationPath

    public init(path: Binding<NavigationPath>, type: HopType) {
        self.type = type
        _path = path
    }
}

extension HopListViewModel {
    func navigateHome() {
        path = .init()
    }
}

import SwiftUI

extension Color {
    init(_ name: String) {
        guard let namedColor = UIColor(named: name, in: Bundle.module, compatibleWith: nil)
        else {
            fatalError("Could not load color from Theme module")
        }
        self.init(namedColor)
    }
}

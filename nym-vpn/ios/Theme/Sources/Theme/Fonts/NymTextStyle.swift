import SwiftUI

public struct NymTextStyle {
    let nymFont: NymFont
    let lineSpacing: CGFloat

    init(nymFont: NymFont, lineSpacing: CGFloat = 0) {
        self.nymFont = nymFont
        self.lineSpacing = lineSpacing
    }
}

extension NymTextStyle {
    public struct Title {
        public struct Large {
            public static var primary: NymTextStyle {
                NymTextStyle(nymFont: .lato(size: 22, weight: .regular))
            }
        }
    }
}

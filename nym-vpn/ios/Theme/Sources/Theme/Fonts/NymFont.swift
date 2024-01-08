import SwiftUI

public enum NymFont {
    case lato(size: CGFloat, weight: LatoWeight)

    public var font: Font {
        switch self {
        case let .lato(size, weight):
            Font.custom("Lato-\(weight.rawValue)", size: size)
        }
    }
}

// MARK: - Weights -

extension NymFont {
    public enum LatoWeight: String, CaseIterable {
        case regular = "Regular"
        case bold = "Bold"
        case semibold = "SemiBold"
    }
}

// MARK: - Register fonts -

extension NymFont {
    public static func register() {
        for latoWeight in NymFont.LatoWeight.allCases {
            let fontName = "Lato-\(latoWeight.rawValue)"
            guard let fontURL = Bundle.module.url(forResource: fontName, withExtension: "ttf") else { continue }
            CTFontManagerRegisterFontsForURL(fontURL as CFURL, .process, nil)
        }
    }
}

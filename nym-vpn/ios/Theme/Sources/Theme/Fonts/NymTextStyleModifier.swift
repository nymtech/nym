import SwiftUI

public struct NymTextStyleModifier: ViewModifier {
    public let textStyle: NymTextStyle

    public init(textStyle: NymTextStyle) {
        self.textStyle = textStyle
    }

    public func body(content: Content) -> some View {
        content
            .font(textStyle.nymFont.font)
            .kerning(textStyle.kerning)
            .lineSpacing(textStyle.lineSpacing)
    }
}

public extension View {
    func textStyle(_ textStyle: NymTextStyle) -> some View {
        modifier(NymTextStyleModifier(textStyle: textStyle))
    }
}

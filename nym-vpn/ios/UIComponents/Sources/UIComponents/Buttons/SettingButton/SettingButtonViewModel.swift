import SwiftUI
import Theme

public struct SettingButtonViewModel {
    let title: String
    let subtitle: String?
    let isSelected: Bool

    public init(title: String, subtitle: String?, isSelected: Bool) {
        self.title = title
        self.subtitle = subtitle
        self.isSelected = isSelected
    }

    var selectionStrokeColor: Color {
        isSelected ? NymColor.primaryOrange : .clear
    }

    var selectionImageName: String {
        isSelected ? "networkSelectedCircle" : "networkCircle"
    }

    var selectionImageColor: Color {
        isSelected ? NymColor.primaryOrange : NymColor.networkButtonCircle
    }
}

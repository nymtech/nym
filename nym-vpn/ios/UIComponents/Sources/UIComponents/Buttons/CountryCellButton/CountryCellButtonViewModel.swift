import SwiftUI
import Theme

public struct CountryCellButtonViewModel {
    public enum CountryCellButtonType {
        case fastest(country: Country)
        case country(country: Country)

        var country: Country {
            switch self {
            case .fastest(let country):
                return country
            case .country(let country):
                return country
            }
        }
    }

    public let boltImageName = "bolt"
    public let selectedTitle = "selected".localizedString
    public let type: CountryCellButtonType
    public let isSelected: Bool

    public init(type: CountryCellButtonType, isSelected: Bool) {
        self.type = type
        self.isSelected = isSelected
    }

    public var title: String {
        switch type {
        case .fastest(let country):
            return "fastest".localizedString + " (\(country.name))"
        case .country(let country):
            return country.name
        }
    }

    public var backgroundColor: Color {
        isSelected ? NymColor.countrySelectionSelectedBackground : .clear
    }
}

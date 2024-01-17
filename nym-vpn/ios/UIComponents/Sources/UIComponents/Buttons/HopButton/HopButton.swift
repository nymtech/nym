import SwiftUI
import Theme

public struct HopButton: View {
    private let hopType: HopType
    private let country: Country

    public init(hopType: HopType, country: Country) {
        self.hopType = hopType
        self.country = country
    }

    public var body: some View {
        StrokeBorderView(strokeTitle: hopType.localizedTitle) {
            HStack {
                FlagImage(countryCode: country.code)

                Text(country.name)
                    .foregroundStyle(NymColor.sysOnSurface)
                    .textStyle(.Body.Large.primary)
                Spacer()
                Image("arrowRight", bundle: .module)
                    .resizable()
                    .frame(width: 24, height: 24)
                    .padding(16)
            }
        }
    }
}

public struct Country {
    public let name: String
    public let code: String

    public init(name: String, code: String) {
        self.name = name
        self.code = code
    }
}

public enum HopType {
    case first
    case last

    var localizedTitle: String {
        switch self {
        case .first:
            "firstHop".localizedString
        case .last:
            "lastHop".localizedString
        }
    }
}

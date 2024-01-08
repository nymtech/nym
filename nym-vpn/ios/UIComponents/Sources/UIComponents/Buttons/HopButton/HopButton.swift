import SwiftUI
import Theme

public struct HopButton: View {
    private let country: Country

    public init(country: Country) {
        self.country = country
    }

    public var body: some View {
        VStack(alignment: .leading) {
            HStack {
                Image(country.code)
                    .resizable()
                    .frame(width: 24, height: 24)
                    .cornerRadius(50)
                    .padding(16)

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
        .padding(0)
        .frame(maxWidth: .infinity, minHeight: 56, maxHeight: 56)
        .cornerRadius(8)
        .overlay {
            RoundedRectangle(cornerRadius: 8)
                .inset(by: 0.5)
                .stroke(Color(red: 0.29, green: 0.27, blue: 0.31), lineWidth: 1)
        }
        .overlay(alignment: .topLeading) {
            Text("firstHop".localizedString)
                .foregroundStyle(NymColor.sysOnSurface)
                .textStyle(.Body.Small.primary)
                .padding(4)
                .background(NymColor.background)
                .position(x: 40, y: 0)
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

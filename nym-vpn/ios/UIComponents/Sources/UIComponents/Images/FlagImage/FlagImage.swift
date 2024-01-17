import SwiftUI

public struct FlagImage: View {
    private let countryCode: String

    public init(countryCode: String) {
        self.countryCode = countryCode
    }

    public var body: some View {
        Image(countryCode)
            .resizable()
            .frame(width: 24, height: 24)
            .cornerRadius(50)
            .padding(16)
    }
}

import SwiftUI
import Theme

public struct ConnectButton: View {
    public init() {}

    public var body: some View {
        HStack {
            Text("Connect")
                .foregroundStyle(NymColor.connectTitle)
                .textStyle(.Label.Huge.primary)
        }
        .frame(maxWidth: .infinity, minHeight: 56, maxHeight: 56)
        .background(NymColor.primaryOrange)
        .cornerRadius(8)
    }
}

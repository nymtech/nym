import SwiftUI
import Theme

public struct StatusButton: View {
    private var config: StatusButtonConfig

    public init(config: StatusButtonConfig) {
        self.config = config
    }

    public var body: some View {
        HStack(alignment: .center, spacing: 10) {
            Text(config.title)
                .foregroundStyle(config.textColor)
                .textStyle(.Label.Huge.primary)
        }
        .padding(.horizontal, 24)
        .padding(.vertical, 16)
        .background(config.backgroundColor)

        .cornerRadius(50)
    }
}

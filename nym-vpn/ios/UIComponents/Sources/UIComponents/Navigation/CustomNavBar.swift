import SwiftUI
import Theme

public struct CustomNavBar: View {
    public let title: String
    public let leftButtonConfig: CustomNavBarButtonConfig?
    public let rightButtonConfig: CustomNavBarButtonConfig?

    public init(
        title: String,
        leftButtonConfig: CustomNavBarButtonConfig? = nil,
        rightButtonConfig: CustomNavBarButtonConfig? = nil
    ) {
        self.title = title
        self.leftButtonConfig = leftButtonConfig
        self.rightButtonConfig = rightButtonConfig
    }

    public var body: some View {
        HStack {
            button(with: leftButtonConfig)
            Spacer()
            Text(title)
                .textStyle(.Title.Large.primary)
            Spacer()
            button(with: rightButtonConfig)
        }
        .frame(height: 64)
        .background {
            NymColor.navigationBarBackground
                .ignoresSafeArea()
        }
    }
}

private extension CustomNavBar {
    @ViewBuilder
    func button(with config: CustomNavBarButtonConfig?) -> some View {
        Button {
            config?.action?()
        } label: {
            if let type = config?.type {
                Image(type.imageName)
                    .tint(NymColor.navigationBarSettingsGear)
            }
        }
        .frame(width: 48, height: 48)
    }
}

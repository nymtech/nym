import SwiftUI
import Theme

public struct CustomNavBar: View {
    public let title: String
    public let leftButton: CustomNavBarButton?
    public let rightButton: CustomNavBarButton?

    public init(
        title: String,
        leftButton: CustomNavBarButton? = nil,
        rightButton: CustomNavBarButton? = nil
    ) {
        self.title = title
        self.leftButton = leftButton
        self.rightButton = rightButton
    }

    public var body: some View {
        HStack {
            leftButton
            Spacer()
            Text(title)
                .foregroundStyle(NymColor.sysOnSurface)
                .textStyle(.Title.Large.primary)
            Spacer()
            rightButton
        }
        .frame(height: 64)
        .background {
            NymColor.navigationBarBackground
                .ignoresSafeArea()
        }
    }
}

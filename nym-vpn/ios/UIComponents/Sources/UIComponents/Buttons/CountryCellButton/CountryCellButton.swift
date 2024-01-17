import SwiftUI
import Theme

public struct CountryCellButton: View {
    private let viewModel: CountryCellButtonViewModel

    public init(viewModel: CountryCellButtonViewModel) {
        self.viewModel = viewModel
    }

    public var body: some View {
        HStack {
            flagOrBoltImage()

            Text(viewModel.title)
                .foregroundStyle(NymColor.sysOnSurface)
                .textStyle(.Body.Large.primary)

            Spacer()
            selectedTitleView()
        }
        .contentShape(
            RoundedRectangle(cornerRadius: 8)
        )

        .frame(width: 360, height: 56, alignment: .center)
        .background(viewModel.backgroundColor)
        .cornerRadius(8)
    }
}

private extension CountryCellButton {
    @ViewBuilder
    func boltImage() -> some View {
        Image(viewModel.boltImageName, bundle: .module)
            .resizable()
            .frame(width: 24, height: 24)
            .foregroundStyle(NymColor.sysOnSurface)
    }

    @ViewBuilder
    func flagOrBoltImage() -> some View {
        switch viewModel.type {
        case .fastest:
            boltImage()
                .padding(.horizontal, 16)
        case .country:
            FlagImage(countryCode: viewModel.type.country.code)
        }
    }

    @ViewBuilder
    func selectedTitleView() -> some View {
        if viewModel.isSelected {
            Text(viewModel.selectedTitle)
                .textStyle(.Label.Small.primary)
                .padding(.trailing, 24)
        }
    }
}

import SwiftUI
import Theme

public struct SearchView: View {
    @ObservedObject private var viewModel: SearchViewModel

    public init(viewModel: SearchViewModel) {
        self.viewModel = viewModel
    }

    public var body: some View {
        StrokeBorderView(strokeTitle: viewModel.strokeTitle) {
            HStack {
                searchImage()
                searchTextfield()
                Spacer()
            }
        }
    }
}

extension SearchView {
    @ViewBuilder
    func searchImage() -> some View {
        Image(viewModel.searchImageName, bundle: .module)
            .resizable()
            .frame(width: 24, height: 24)
            .cornerRadius(50)
            .padding(16)
    }

    @ViewBuilder
    func searchTextfield() -> some View {
        ZStack(alignment: .leading) {
            if viewModel.searchText.isEmpty {
                Text(viewModel.searchCountryTitle)
                    .foregroundStyle(NymColor.sysOutline)
                    .textStyle(.Body.Large.primary)
            }
            TextField("", text: $viewModel.searchText)
                .foregroundStyle(NymColor.sysOnSurface)
                .textStyle(.Body.Large.primary)
        }
    }
}

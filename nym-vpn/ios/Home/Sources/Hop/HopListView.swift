import SwiftUI
import Theme
import UIComponents

public struct HopListView: View {
    private let viewModel: HopListViewModel

    public init(viewModel: HopListViewModel) {
        self.viewModel = viewModel
    }

    public var body: some View {
        VStack {
            navbar()
            Spacer()
                .frame(height: 24)

            searchView()
            Spacer()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background {
            NymColor.background
                .ignoresSafeArea()
        }
    }
}

private extension HopListView {
    @ViewBuilder
    func navbar() -> some View {
        CustomNavBar(
            title: viewModel.type.selectHopLocalizedTitle,
            leftButton: CustomNavBarButton(type: .back, action: {})
        )
    }

    @ViewBuilder
    func searchView() -> some View {
        SearchView(viewModel: SearchViewModel())
            .padding(.horizontal, 16)
    }
}

public struct HopListViewModel {
    public let type: HopType

    public init(type: HopType) {
        self.type = type
    }
}

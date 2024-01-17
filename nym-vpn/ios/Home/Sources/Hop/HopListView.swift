import SwiftUI
import Theme
import UIComponents

public struct HopListView: View {
    private let viewModel: HopListViewModel

    public init(viewModel: HopListViewModel) {
        self.viewModel = viewModel
    }

    public var body: some View {
        VStack(spacing: 0) {
            navbar()
            Spacer()
                .frame(height: 24)

            countryButton()
            Spacer()
                .frame(height: 24)

            countryButton2()
            Spacer()
                .frame(height: 24)

            searchView()
            Spacer()
                .frame(height: 24)

            availableCountryList()
        }
        .navigationBarBackButtonHidden(true)
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
            leftButton: CustomNavBarButton(type: .back, action: { viewModel.navigateHome() })
        )
    }

    @ViewBuilder
    func countryButton() -> some View {
        CountryCellButton(
            viewModel: CountryCellButtonViewModel(
                type: .fastest(
                    country: Country(name: "Germany", code: "de")
                ),
                isSelected: false
            )
        )
        .padding(.horizontal, 15)
    }

    @ViewBuilder
    func countryButton2() -> some View {
        CountryCellButton(
            viewModel: CountryCellButtonViewModel(
                type: .country(
                    country: Country(name: "Germany", code: "de")
                ),
                isSelected: true
            )
        )
        .padding(.horizontal, 15)
    }

    @ViewBuilder
    func searchView() -> some View {
        SearchView(viewModel: SearchViewModel())
            .padding(.horizontal, 40)
    }

    @ViewBuilder
    func availableCountryList() -> some View {
        ScrollView {
            switzerland()
            germany()
            switzerland()
            germany()
            switzerland()
            germany()
            switzerland()
            germany()
            switzerland()
            germany()
            switzerland()
            germany()
            switzerland()
            germany()
            switzerland()
            germany()
            switzerland()
            germany()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .ignoresSafeArea(.all)
    }

    @ViewBuilder
    func germany() -> some View {
        CountryCellButton(
            viewModel: CountryCellButtonViewModel(
                type: .country(
                    country: Country(name: "Germany", code: "de")
                ),
                isSelected: false
            )
        )
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .ignoresSafeArea(.all)
    }

    @ViewBuilder
    func switzerland() -> some View {
        CountryCellButton(
            viewModel: CountryCellButtonViewModel(
                type: .country(
                    country: Country(name: "Switzerland", code: "ch")
                ),
                isSelected: false
            )
        )
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .ignoresSafeArea(.all)
    }
}

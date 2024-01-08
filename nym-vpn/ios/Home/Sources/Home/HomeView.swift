import SwiftUI
import UIComponents
import Theme

public struct HomeView: View {
    @ObservedObject private var viewModel = HomeViewViewModel(selectedNetwork: .mixnet)
    public init() {}

    public var body: some View {
        VStack {
            navbar()
            statusAreaSection()
            networkSection()
            connectionSection()
            connectButton()
            Spacer()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background {
            NymColor.background
                .ignoresSafeArea()
        }
    }
}

private extension HomeView {
    @ViewBuilder
    func navbar() -> some View {
        CustomNavBar(
            title: "NymVPN".localizedString,
            leftButton: CustomNavBarButton(type: .empty, action: {}),
            rightButton: CustomNavBarButton(type: .settingsGear, action: {})
        )
        Spacer()
            .frame(height: 50)
    }

    @ViewBuilder
    func statusAreaSection() -> some View {
        StatusButton(config: .disconnected)
        Spacer()
            .frame(height: 8)

        StatusInfoView()
        Spacer()
            .frame(height: 24)
    }

    @ViewBuilder
    func networkSection() -> some View {
        HStack {
            Text("selectNetwork".localizedString)
                .textStyle(.Title.Medium.primary)
            Spacer()
        }
        .padding(.horizontal, 16)
        Spacer()
            .frame(height: 24)

        NetworkButton(viewModel: NetworkButtonViewModel(type: .mixnet, selectedNetwork: $viewModel.selectedNetwork))
            .padding(EdgeInsets(top: 0, leading: 16, bottom: 16, trailing: 16))
            .onTapGesture {
                viewModel.selectedNetwork = .mixnet
            }

        NetworkButton(viewModel: NetworkButtonViewModel(type: .wireguard, selectedNetwork: $viewModel.selectedNetwork))
            .padding(.horizontal, 16)
            .onTapGesture {
                viewModel.selectedNetwork = .wireguard
            }
        Spacer()
            .frame(height: 32)
    }

    @ViewBuilder
    func connectionSection() -> some View {
        HStack {
            Text("connectTo".localizedString)
                .foregroundStyle(NymColor.sysOnSurfaceWhite)
                .textStyle(.Title.Medium.primary)
            Spacer()
        }
        .padding(.horizontal, 16)

        Spacer()
            .frame(height: 24)

        VStack {
            HopButton(country: Country(name: "Germany", code: "de"))
            Spacer()
                .frame(height: 24)
            HopButton(country: Country(name: "Switzerland", code: "ch"))
        }
        .padding(.horizontal, 16)

        Spacer()
            .frame(height: 32)
    }

    @ViewBuilder
    func connectButton() -> some View {
        ConnectButton()
            .padding(.horizontal, 16)
    }
}

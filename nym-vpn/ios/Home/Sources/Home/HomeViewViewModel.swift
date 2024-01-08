import SwiftUI
import UIComponents

public class HomeViewViewModel: ObservableObject {
    @Published var selectedNetwork: NetworkButtonViewModel.ButtonType

    public init(selectedNetwork: NetworkButtonViewModel.ButtonType) {
        self.selectedNetwork = selectedNetwork
    }
}

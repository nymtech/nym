import SwiftUI

public class HomeFlowState: ObservableObject {
    @Published var path = NavigationPath()
    @Published var presentedItem: HomeLink?
    @Published var coverItem: HomeLink?
}

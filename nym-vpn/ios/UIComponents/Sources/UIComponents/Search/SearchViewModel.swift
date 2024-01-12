import Foundation

public final class SearchViewModel: ObservableObject {
    let strokeTitle = "search".localizedString
    let searchCountryTitle = "searchCountry".localizedString
    let searchImageName = "searchIcon"

    @Published var searchText = ""

    public init() {}
}

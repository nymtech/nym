import SwiftUI

public final class AppSettings: ObservableObject {
    public static let shared = AppSettings()

    @AppStorage("currentTheme") public var currentTheme: Theme = .automatic
}

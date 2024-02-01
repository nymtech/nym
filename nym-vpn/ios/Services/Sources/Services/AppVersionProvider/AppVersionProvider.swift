import Foundation

public enum AppVersionProvider {
    public static func appVersion(in bundle: Bundle = .main) -> String {
        guard let version = bundle.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String
        else {
            fatalError("Missing CFBundleShortVersionString")
        }
        return version
    }
}

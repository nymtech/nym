import NetworkExtension

public final class Tunnel: ObservableObject {
    public var tunnel: NETunnelProviderManager
    @Published public var isEnabled: Bool

    private var observers = [AnyObject]()

    public init(tunnel: NETunnelProviderManager) {
        self.tunnel = tunnel
        isEnabled = tunnel.isEnabled

        let configurationChangeNotification = NotificationCenter.default.addObserver(
            forName: .NEVPNConfigurationChange,
            object: tunnel,
            queue: .main
        ) { [weak self] _ in
            print("Tunnel isEnabled: \(tunnel.isEnabled)")
            self?.isEnabled = tunnel.isEnabled
        }
        observers.append(configurationChangeNotification)
    }
}

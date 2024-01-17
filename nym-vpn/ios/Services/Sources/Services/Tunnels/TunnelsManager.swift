import NetworkExtension
import OSLog

public final class TunnelsManager: ObservableObject {
    public static let shared = TunnelsManager()

    public var currentTunnel: Tunnel?

    private var observers = [AnyObject]()

    private init() {
        setup()
    }

    public func loadConfigurations() {
        NETunnelProviderManager.loadAllFromPreferences { [weak self] managers, error in
            print("1. Loading VPN Configurations")
            if let error {
                print("Error: \(String(describing: error))")
            }

            managers?.forEach {
                print("Found VPN Configuration")
                print("\($0)")
                self?.currentTunnel = Tunnel(tunnel: $0)
            }
        }
    }

    public func test() {
        loadConfigurations()
        os_log("2 Starting test")
        if currentTunnel == nil {
            let manager = createTestManager()
            manager.saveToPreferences { error in
                if error == nil {
                    print("3 Added config Successfully")
                } else {
                    print("3 Failure to add config")
                }
            }
        } else {
            print("3 Current tunnel already exists")
            currentTunnel?.tunnel.isEnabled = true
        }

        print("4 Connecting")
        connect()
    }

    public func connect() {
        do {
            let options = [
                NEVPNConnectionStartOptionUsername: "john",
                NEVPNConnectionStartOptionPassword: "password"
            ] as [String: NSObject]

            try currentTunnel?.tunnel.connection.startVPNTunnel(options: options)
        } catch {
            print("FAILED to connect: \(error)")
        }
    }

    public func disconnect() {
        print("5 Disconnecting")
        //        currentTunnel?.tunnel.connection.stopVPNTunnel()

        NEVPNManager.shared().loadFromPreferences { [weak self] error in
            if let error {
                print("Error: \(error)")
            }
            self?.currentTunnel?.tunnel.connection.stopVPNTunnel()
            NEVPNManager.shared().connection.stopVPNTunnel()
        }
    }
}

private extension TunnelsManager {
    func createTestManager() -> NETunnelProviderManager {
        let manager = NETunnelProviderManager()
        manager.localizedDescription = "NymVPN Mixnet"

        let tunnelConfiguration = NETunnelProviderProtocol()
        tunnelConfiguration.providerBundleIdentifier = "net.nymtech.vpn.network-extension"
        tunnelConfiguration.serverAddress = "127.0.0.1:4009"
        tunnelConfiguration.providerConfiguration = [:]

        manager.protocolConfiguration = tunnelConfiguration
        manager.isEnabled = true
        return manager
    }
}

private extension TunnelsManager {
    func setup() {
        registerNotifications()
    }

    func registerNotifications() {
        let statusDidChangeNotification = NotificationCenter.default.addObserver(
            forName: .NEVPNStatusDidChange,
            object: nil,
            queue: .main
        ) { status in
            print("VPN Status: \(status)")
        }
        observers.append(statusDidChangeNotification)
    }
}

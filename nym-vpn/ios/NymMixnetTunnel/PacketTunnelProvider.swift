import NetworkExtension
import OSLog

class PacketTunnelProvider: NEPacketTunnelProvider {
    override func startTunnel(options: [String: NSObject]?, completionHandler: @escaping (Error?) -> Void) {
        NSLog("🏁 Starting tunnel with options: \(options ?? [:])")
        os_log("🏁 Starting tunnel with options: \(options ?? [:])")

        fetchData()
        completionHandler(nil)
    }

    func fetchData() {
        // Do not use the NSTimer here that will not run in background
        let popTime = DispatchTime.now() + DispatchTimeInterval.seconds(Int(1))
        DispatchQueue.global(qos: .background).asyncAfter(deadline: popTime) { [weak self] in
            // Fetch your data from server and generate local notification by using UserNotifications framework
            self?.doSomeStuff()
            self?.fetchData()
        }
    }

    @objc func doSomeStuff() {
        NSLog("🔥 ROKAS TIMER TIC TAK")
        os_log("🔥 ROKAS TIMER TIC TAK")
    }

    override func stopTunnel(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        NSLog("🛑 Rokas stopping tunnel")
        completionHandler()
    }

    override func handleAppMessage(_ messageData: Data, completionHandler: ((Data?) -> Void)?) {

        if let handler = completionHandler {
            handler(messageData)
        }
    }

    override func sleep(completionHandler: @escaping () -> Void) {
        completionHandler()
    }

    override func wake() {}
}

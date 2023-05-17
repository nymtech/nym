//
//  RustSocks5.swift
//  Socks5
//
//  Created by Jedrzej Stuczynski on 12/05/2023.
//

func start_callback(clientAddress: UnsafeMutablePointer<CChar>?) {
    let swift_string = String(cString: clientAddress!)
    rust_free_string(clientAddress)
    print("the client is now alive! And its address is \(swift_string)")
}

func shutdown_callback() {
    print("the client is now dead")
}

class RustSocks5 {
//    func runForever(storageDirectory, serviceProvider: String) {
//        let start_cb: @convention(c) () -> Void = start_callback;
//        let shutdown_cb: @convention(c) () -> Void = shutdown_callback;
//
//        blocking_run_client(serviceProvider, start_cb, shutdown_cb)
//    }
    
    func startClient(storageDirectory: String, serviceProvider: String) {
        let start_cb: @convention(c) (UnsafeMutablePointer<CChar>?) -> Void = start_callback;
        let shutdown_cb: @convention(c) () -> Void = shutdown_callback;
        
        start_client(storageDirectory, serviceProvider, start_cb, shutdown_cb)
    }
    
    func stopClient() {
        stop_client()
    }

    
//    func addStuff(to: String) -> String {
//        let result = foomp(to)
//        let swift_result = String(cString: result!)
//        free_foomp(UnsafeMutablePointer(mutating: result))
//        return swift_result
//    }
//
//    func addStuffWithCallback(to: String) -> String {
//        let f: @convention(c) () -> Void = callback
//        let result = invoke_foomp_with_callback(to, f)
//
//
//        let swift_result = String(cString: result!)
//        free_foomp(UnsafeMutablePointer(mutating: result))
//        return swift_result
//    }
}

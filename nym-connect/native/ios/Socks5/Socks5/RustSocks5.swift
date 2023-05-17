//
//  RustSocks5.swift
//  Socks5
//
//  Created by Jedrzej Stuczynski on 12/05/2023.
//

import SwiftUI


class RustSocks5: ObservableObject {
    @Published var operationInProgress = false
    @Published var clientAddress = ""
    @Published var connected = false
    @Published var status: ClientState = CLIENT_STATE_UNKNOWN
    
    func onConnect(clientAddress: UnsafeMutablePointer<CChar>?) {
        print("connected callback got called!")
        let swift_string = String(cString: clientAddress!)
        rust_free_string(clientAddress)
        print("the client is now alive! And its address is \(swift_string)")
        
        DispatchQueue.main.async{
            self.status = CLIENT_STATE_CONNECTED
            self.connected = true
            self.operationInProgress = false
            self.clientAddress = swift_string
        }
    }
    
    func onShutdown() {
        print("shutdown callback got called!")
        
        DispatchQueue.main.async{
            self.status = CLIENT_STATE_DISCONNECTED
            self.connected = false
            self.operationInProgress = false
        }
    }
    
    
    func startClient(storageDirectory: String, serviceProvider: String) {
        let this1 = UnsafeMutableRawPointer(Unmanaged.passRetained(self).toOpaque())
        let startCb: @convention(c) (UnsafeMutableRawPointer?, UnsafeMutablePointer<CChar>?) -> Void = {
            let socks: RustSocks5 = Unmanaged.fromOpaque($0!).takeRetainedValue()
            socks.onConnect(clientAddress: $1)
        }
        
        let this2 = UnsafeMutableRawPointer(Unmanaged.passRetained(self).toOpaque())
        let shutdownCb: @convention(c) (UnsafeMutableRawPointer?) -> Void = {
            let socks: RustSocks5 = Unmanaged.fromOpaque($0!).takeRetainedValue()
            socks.onShutdown()
        }

        let fn_start = RefDynFnMut1_void_char_ptr(env_ptr: this1, call: startCb)
        let fn_shutdown = RefDynFnMut0_void(env_ptr: this2, call: shutdownCb)
        
        
        start_client(storageDirectory, serviceProvider, fn_start, fn_shutdown)
    }
    
    func stopClient() {
        stop_client()
    }
    
    func resetConfig(storageDirectory: String) {
        reset_client_data(storageDirectory)
    }
}

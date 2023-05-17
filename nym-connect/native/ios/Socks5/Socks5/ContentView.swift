//
//  ContentView.swift
//  Socks5
//
//  Created by Jedrzej Stuczynski on 12/05/2023.
//

import SwiftUI

let NYM_CLIENT_STORAGE_DIR = "/.nym/socks5-clients";

struct ContentView: View {
    @StateObject private var socksWrapper: RustSocks5 = RustSocks5()
    
    func clientStoreDirectory() -> String {
        let dirPaths = NSSearchPathForDirectoriesInDomains(.documentDirectory, .userDomainMask, true)
        let client_store_dir = dirPaths[0] + NYM_CLIENT_STORAGE_DIR
        return client_store_dir
    }

    func connect() {
        print("connecting (swift)...")
        socksWrapper.operationInProgress = true

        let client_store_dir = self.clientStoreDirectory()
        socksWrapper.startClient(
            storageDirectory: client_store_dir,
            serviceProvider: "4z4iw9NLRgMok2MPFEGoiwrmHuDY6kRVDUQRp2dXGLQm.69av5mWZmaMK4bHo3GV6Cu7B8zuMT2mv2E22f8GkRMgk@DF4TE7V8kJkttMvnoSVGnRFFRt6WYGxxiC2w1XyPQnHe")
        
    }
              
    
    func disconnect() {
        print("disconnecting (swift)...")
        socksWrapper.operationInProgress = true

        socksWrapper.stopClient()
    }
    
    func reset() {
        print("resetting (swift)...")
        
        let client_store_dir = self.clientStoreDirectory()
        socksWrapper.resetConfig(storageDirectory: client_store_dir)
    }
        
    
    var body: some View {
        VStack {
            Image(systemName: "globe")
                .imageScale(.large)
                .foregroundColor(.accentColor)
            
            
            HStack {
                Button(action: connect) {
                    Text("connect")
                }.disabled(socksWrapper.connected || socksWrapper.operationInProgress)
                Button(action: disconnect) {
                    Text("disconnect")
                }.disabled(!socksWrapper.connected || socksWrapper.operationInProgress)
                Button(action: reset) {
                    Text("reset").foregroundColor(.red)
                }.disabled(true)
            }
            .buttonStyle(.bordered)
            
            if socksWrapper.operationInProgress {
                ProgressView().progressViewStyle(CircularProgressViewStyle())
            }
            
            
            Text("status: \(socksWrapper.status.description)")
            Text("address: \(socksWrapper.clientAddress)")
        }
        .padding()
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}

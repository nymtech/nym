//
//  ContentView.swift
//  Socks5
//
//  Created by Jedrzej Stuczynski on 12/05/2023.
//

import SwiftUI

let NYM_CLIENT_STORAGE_DIR = "/.nym/socks5-clients";

func clientStoreDirectory() -> String {
    let dirPaths = NSSearchPathForDirectoriesInDomains(.documentDirectory, .userDomainMask, true)
    let client_store_dir = dirPaths[0] + NYM_CLIENT_STORAGE_DIR
    return client_store_dir
}

struct ContentView: View {
    @State private var connected: Bool = false;
    @StateObject private var socksWrapper: RustSocks5 = RustSocks5();
    @State private var serviceProvider: String?;
    
    func connect() {
        print("connecting (swift)...")
        socksWrapper.operationInProgress = true
        
        var targetServiceProvider: String? = "4z4iw9NLRgMok2MPFEGoiwrmHuDY6kRVDUQRp2dXGLQm.69av5mWZmaMK4bHo3GV6Cu7B8zuMT2mv2E22f8GkRMgk@DF4TE7V8kJkttMvnoSVGnRFFRt6WYGxxiC2w1XyPQnHe"
        if socksWrapper.serviceProvider != nil {
            // explicitly don't pass anything because we don't have to
            // note: you can't overwrite existing provider. you'd have to reset everything instead.
            targetServiceProvider = nil
        }
        
        let client_store_dir = clientStoreDirectory()
        socksWrapper.startClient(
            storageDirectory: client_store_dir,
            serviceProvider: targetServiceProvider)
    }

    
    func disconnect() {
        print("disconnecting (swift)...")
        socksWrapper.operationInProgress = true

        socksWrapper.stopClient()
    }
    
    func reset() {
        print("resetting (swift)...")
        
        let client_store_dir = clientStoreDirectory()
        socksWrapper.resetConfig(storageDirectory: client_store_dir)
    }

    func getStatusText() -> String {
        if socksWrapper.operationInProgress {
            return "Please wait..."
        }
        return socksWrapper.connected ? "Connected to the mixnet" : "Not connected to the Nym mixnet"
    }
    
    func getStatusColor() -> Color {
        if socksWrapper.operationInProgress {
            return .secondary
        }
        return socksWrapper.connected ? .green : .primary
    }

    func getStatusImage() -> String {
        if socksWrapper.operationInProgress {
            return "arrow.2.circlepath.circle"
        }
        return socksWrapper.connected ? "checkmark.circle" : "exclamationmark.circle"
    }


    var body: some View {
        VStack {
            Toggle(isOn: $connected) {
                Label(getStatusText(), systemImage: getStatusImage()).foregroundColor(getStatusColor())
                    .fontWeight(.medium)
            }.disabled(socksWrapper.operationInProgress)
                .onChange(of: connected, perform: { value in
                    if value {
                        connect()
                    } else {
                        disconnect()
                    }
                })

            Text("NymConnect is not a VPN. It starts a SOCKS5 proxy on your device that you can connect apps that support SOCKS5 so that their traffic is sent across the Nym Mixnet.")
                .multilineTextAlignment(.leading)
                .padding(.top)
            Text("Follow these instructions to configure Telegram to use NymConnect:")
                .font(.subheadline)
                .multilineTextAlignment(.center)
                .padding(.top)
            Text("TODO")
                .padding(.top)

            Spacer()
            Image(systemName: "globe")
                .imageScale(.large)
                .foregroundColor(.accentColor)
            
            

            DeleteButton().environmentObject(socksWrapper).buttonStyle(.bordered)
            
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



struct DeleteButton: View {
  @EnvironmentObject var socksWrapper: RustSocks5
  @State private var isPresentingConfirm: Bool = false
    
  var body: some View {
    Button("Reset", role: .destructive) {
      isPresentingConfirm = true
    }
    .confirmationDialog("Are you sure?",
      isPresented: $isPresentingConfirm) {
        Button("Delete client configuration?", role: .destructive) {
            let storageDirectory = clientStoreDirectory()
            socksWrapper.resetConfig(storageDirectory: storageDirectory)
        }
    } message: {
      Text("You cannot undo this action!")
    }
  }
}

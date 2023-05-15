//
//  ContentView.swift
//  Socks5
//
//  Created by Jedrzej Stuczynski on 12/05/2023.
//

import SwiftUI


struct ContentView: View {
    @State private var connected: Bool = false
    @State private var status: ClientState = CLIENT_STATE_UNKNOWN
    

    
    func connect() {
        let foomper = RustSocks5()
        let res = foomper.addStuffWithCallback(to: "foomp")
        print(res)
        
//        foomper.runForever(serviceProvider: "4z4iw9NLRgMok2MPFEGoiwrmHuDY6kRVDUQRp2dXGLQm.69av5mWZmaMK4bHo3GV6Cu7B8zuMT2mv2E22f8GkRMgk@DF4TE7V8kJkttMvnoSVGnRFFRt6WYGxxiC2w1XyPQnHe")
//        print("\(foomper.addStuff(to: "world"))")
//
//        print("connecting (swift)")
        //            let rustSocks5 = RustSocks5()
        //            rustSocks5.runForever(serviceProvider: "my-service-provider-address")
        connected = true
        
    }
              
    
    func disconnect() {
        print("disconnecting (swift)")
        connected = false
    }
    
    
    
    var body: some View {
        VStack {
            Image(systemName: "globe")
                .imageScale(.large)
                .foregroundColor(.accentColor)
            
            HStack {
                Button(action: connect) {
                    Text("connect")
                }.disabled(connected)
                Button(action: disconnect) {
                    Text("disconnect")
                }.disabled(!connected)
            }
            .buttonStyle(.borderedProminent)
            Text("status: \(status.description)")
        }
        .padding()
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}

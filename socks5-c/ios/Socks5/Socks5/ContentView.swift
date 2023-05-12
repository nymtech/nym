//
//  ContentView.swift
//  Socks5
//
//  Created by Jedrzej Stuczynski on 12/05/2023.
//

import SwiftUI

struct ContentView: View {
    var body: some View {
        VStack {
            Image(systemName: "globe")
                .imageScale(.large)
                .foregroundColor(.accentColor)
            Text("Hello, world!")
        }
        .padding()
        .onAppear{
            let rustSocks5 = RustSocks5()
            rustSocks5.runForever(serviceProvider: "my-service-provider-address")
        }
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView()
    }
}

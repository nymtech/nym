# Socks5-c

## Setting up dummy socks5 client in iOS (from scratch)

Based on https://mozilla.github.io/firefox-browser-architecture/experiments/2017-09-06-rust-on-ios.html

1. Install xcode, iOS simulator, etc. (`xcode-select --install`)
2. add rust targets: `rustup target add aarch64-apple-ios x86_64-apple-ios`
3. get `cargo-lipo`: `cargo install cargo-lipo`
4. build our library file. From inside `cargo` run `cargo lipo --release`
5. go to **General** tab of the xcode project setting and under **Linked Frameworks and Libraries** import `/cargo/target/universal/release/libsocks5_c.a`
6. link `libresolv.tbd`: in **Linked Frameworks and Libraries** search for and select `libresolv.tbd`
7. import `socks5_c.h` header file into the project: `File -> Add Files to "..."`
8. create bridging header: make new `Socks5-Bridging-Header.h` file and put the following inside:
```
    #ifndef Greetings_Bridging_Header_h
    #define Greetings_Bridging_Header_h
    
    #import "socks5_c.h"
    
    #endif
```
9. tell xcode about the bridging header: go to **Build Settings** tab and set `Objective-C Bridging Header` to `$(PROJECT_DIR)/Socks5/Socks5-Bridging-Header.h`
10. tell xcode where to look for libraries for linking: go to **Build Settings** tab and amend the Library Search Paths option value to `$(PROJECT_DIR)/../../cargo/target/universal/release`
11. create a swift file to actually call our code (`RustSocks5.swift`)
```swift
// this is an example

class RustSocks5 {
    func runForever(serviceProvider: String) {
        run_client(serviceProvider)
    }
}
 
```
12. call the new code however you want. I did it by setting `onAppear`:
```swift
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
```

13. note: the above requires passing a service provider address. The easiest (for testing) would be to just run a local network requester with **open proxy** and passing that address. 
14. fix iOS simulator: go to **Build Settings** and add _Any iOS Simulator SDK_ with value `arm64` inside Excluded Architecture.
15. start the simulator!
16. test it by sending some request, like `curl -x socks5h://127.0.0.1:1080 https://nymtech.net`
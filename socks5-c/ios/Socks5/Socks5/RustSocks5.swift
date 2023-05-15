//
//  RustSocks5.swift
//  Socks5
//
//  Created by Jedrzej Stuczynski on 12/05/2023.
//

func callback() {
    print("this is a callback!")
}

class RustSocks5 {

    
    func runForever(serviceProvider: String) {
        run_client(serviceProvider)
    }
    
    func addStuff(to: String) -> String {
        let result = foomp(to)
        let swift_result = String(cString: result!)
        free_foomp(UnsafeMutablePointer(mutating: result))
        return swift_result
    }
    
    func addStuffWithCallback(to: String) -> String {
        let f: @convention(c) () -> Void = callback
        let result = invoke_foomp_with_callback(to, f)
  
        
        let swift_result = String(cString: result!)
        free_foomp(UnsafeMutablePointer(mutating: result))
        return swift_result
    }
}

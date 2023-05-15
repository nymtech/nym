//
//  RustSocks5.swift
//  Socks5
//
//  Created by Jedrzej Stuczynski on 12/05/2023.
//

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
}

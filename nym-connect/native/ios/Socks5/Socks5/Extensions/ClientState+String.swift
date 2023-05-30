//
//  ClientState+String.swift
//  Socks5
//
//  Created by Jedrzej Stuczynski on 15/05/2023.
//

extension ClientState: CustomStringConvertible {
    public var description: String {
        switch self {
        case CLIENT_STATE_UNINITIALISED:
            return "uninitialised"
        case CLIENT_STATE_CONNECTED:
            return "connected"
        case CLIENT_STATE_DISCONNECTED:
            return "disconnected"
        default:
            fatalError("invalid client state")
        }
    }
}

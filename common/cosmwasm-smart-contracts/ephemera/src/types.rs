// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

#[cw_serde]
pub struct JsonPeerInfo {
    /// The cosmos address of the peer, used in interacting with the chain.
    pub cosmos_address: Addr,
    /// The TCP/IP address of the peer.
    /// Expected formats:
    /// 1. `<IP>:<PORT>`
    /// 2. `/ip4/<IP>/tcp/<PORT>` - this is the format used by libp2p multiaddr
    pub ip_address: String,
    ///Serialized public key.
    pub public_key: String,
}

impl JsonPeerInfo {
    #[must_use]
    pub fn new(cosmos_address: Addr, ip_address: String, public_key: String) -> Self {
        Self {
            cosmos_address,
            ip_address,
            public_key,
        }
    }
}

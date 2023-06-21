// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema)]
pub struct JsonPeerInfo {
    /// The cosmos address of the peer, used in interacting with the chain.
    pub cosmos_address: Addr,
    /// The TCP/IP address of the peer.
    /// Expected formats:
    /// 1. `<IP>:<PORT>`
    /// 2. `/ip4/<IP>/tcp/<PORT>` - this is the format used by libp2p multiaddr
    pub ip_address: String,
    ///Serialized public key.
    ///
    /// # Converting to string and back example
    ///```
    /// use ephemera::crypto::{EphemeraKeypair, EphemeraPublicKey, Keypair, PublicKey};
    ///
    /// let public_key = Keypair::generate(None).public_key();
    ///
    /// let public_key_str = public_key.to_string();
    ///
    /// let public_key_parsed = public_key_str.parse::<PublicKey>().unwrap();
    ///
    /// assert_eq!(public_key, public_key_parsed);
    /// ```
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

// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use defguard_wireguard_rs::{host::Peer, key::Key};
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Hosts {
    host_v4: defguard_wireguard_rs::host::Host,
    host_v6: defguard_wireguard_rs::host::Host,
}

impl Hosts {
    pub fn new(
        host_v4: defguard_wireguard_rs::host::Host,
        host_v6: defguard_wireguard_rs::host::Host,
    ) -> Self {
        Self { host_v4, host_v6 }
    }

    pub fn get(&self, key: &Key) -> Option<&Peer> {
        self.host_v4
            .peers
            .get(key)
            .or_else(|| self.host_v6.peers.get(key))
    }
}

// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::filter;
use nymsphinx::addressing::nodes::NymNodeRoutingAddress;
use sphinx::route::Node as SphinxNode;
use std::convert::TryInto;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Client {
    pub pub_key: String,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub location: String,
    pub client_listener: SocketAddr,
    pub mixnet_listener: SocketAddr,
    pub pub_key: String,
    pub registered_clients: Vec<Client>,
    pub last_seen: u64,
    pub version: String,
}

impl Node {
    pub fn get_pub_key_bytes(&self) -> [u8; 32] {
        let mut key_bytes = [0; 32];
        bs58::decode(&self.pub_key).into(&mut key_bytes).unwrap();
        key_bytes
    }
}

impl filter::Versioned for Node {
    fn version(&self) -> String {
        self.version.clone()
    }
}

impl Into<SphinxNode> for Node {
    fn into(self) -> SphinxNode {
        let node_address_bytes = NymNodeRoutingAddress::from(self.mixnet_listener)
            .try_into()
            .unwrap();
        let key_bytes = self.get_pub_key_bytes();
        let key = sphinx::key::new(key_bytes);

        SphinxNode::new(node_address_bytes, key)
    }
}

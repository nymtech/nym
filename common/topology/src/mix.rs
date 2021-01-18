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
use crypto::asymmetric::{encryption, identity};
use nymsphinx_addressing::nodes::NymNodeRoutingAddress;
use nymsphinx_types::Node as SphinxNode;
use std::convert::TryInto;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Node {
    pub location: String,
    pub host: SocketAddr,
    pub identity_key: identity::PublicKey,
    pub sphinx_key: encryption::PublicKey, // TODO: or nymsphinx::PublicKey? both are x25519
    pub layer: u64,
    pub registration_time: i64,
    pub reputation: i64,
    pub version: String,
}

impl filter::Versioned for Node {
    fn version(&self) -> String {
        self.version.clone()
    }
}

impl<'a> From<&'a Node> for SphinxNode {
    fn from(node: &'a Node) -> Self {
        let node_address_bytes = NymNodeRoutingAddress::from(node.host).try_into().unwrap();

        SphinxNode::new(node_address_bytes, (&node.sphinx_key).into())
    }
}

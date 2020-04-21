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
use crypto::identity::MixIdentityPublicKey;
use nymsphinx::addressing::nodes::NymNodeRoutingAddress;
use nymsphinx::chunking::split_and_prepare_payloads;
use nymsphinx::{
    delays, Destination, DestinationAddressBytes, Node, NodeAddressBytes, SphinxPacket,
    IDENTIFIER_LENGTH,
};
use serde::{Deserialize, Serialize};
use serde_json;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::net::SocketAddr;
use std::time::Duration;
use wasm_bindgen::prelude::*;

mod utils;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[derive(Serialize, Deserialize)]
pub struct JsonRoute {
    nodes: Vec<NodeData>,
}

#[wasm_bindgen]
#[derive(Clone, Serialize, Deserialize)]
pub struct NodeData {
    address: String,
    public_key: String,
}
/// Creates a Sphinx packet for use in JavaScript applications, using wasm.
///
/// The `wasm-pack build` command will cause this to output JS bindings and a
/// wasm executable in the `pkg/` directory.
///
/// Message chunking is currently not implemented. If the message exceeds the
/// capacity of a single Sphinx packet, the extra information will be discarded.
#[wasm_bindgen]
pub fn create_sphinx_packet(raw_route: &str, msg: &str, destination: &str) -> Vec<u8> {
    utils::set_panic_hook(); // nicer js errors.

    let route = sphinx_route_from(raw_route);

    let average_delay = Duration::from_secs_f64(0.1);
    let delays = delays::generate_from_average_duration(route.len(), average_delay);
    let dest_bytes = DestinationAddressBytes::from_base58_string(destination.to_owned());
    let dest = Destination::new(dest_bytes, [4u8; IDENTIFIER_LENGTH]);
    let message = split_and_prepare_payloads(&msg.as_bytes()).pop().unwrap();
    let sphinx_packet = match SphinxPacket::new(message, &route, &dest, &delays).unwrap() {
        SphinxPacket { header, payload } => SphinxPacket { header, payload },
    };

    payload(sphinx_packet, route)
}

/// Concatenate the first mix address bytes with the Sphinx packet.
///
/// The Nym gateway node has no idea what is inside the Sphinx packet, or where
/// it should send a packet it receives. So we prepend the packet with the
/// address bytes of the first mix inside the packet, so that the gateway can
/// forward the packet to it.
fn payload(sphinx_packet: SphinxPacket, route: Vec<Node>) -> Vec<u8> {
    let packet = sphinx_packet.to_bytes();
    let first_mix_address = route.first().unwrap().clone().address.to_bytes().to_vec();

    first_mix_address
        .into_iter()
        .chain(packet.into_iter())
        .collect()
}

/// Attempts to create a Sphinx route, which is a `Vec<sphinx::Node>`, from a
/// JSON string.
///
/// # Panics
///
/// This function panics if the supplied `raw_route` json string can't be
/// extracted to a `JsonRoute`.
///
/// # Panics
///
/// This function panics if `JsonRoute.nodes` doesn't contain at least 1
/// node.
///
fn sphinx_route_from(raw_route: &str) -> Vec<Node> {
    let json_route: JsonRoute = serde_json::from_str(raw_route).unwrap();

    assert!(
        json_route.nodes.len() > 0,
        "Sphinx packet must route to at least one mixnode."
    );

    let mut sphinx_route: Vec<Node> = vec![];
    for node_data in json_route.nodes.iter() {
        let x = Node::try_from(node_data.clone()).expect("Malformed NodeData");
        sphinx_route.push(x);
    }
    sphinx_route
}

impl TryFrom<NodeData> for Node {
    type Error = ();

    fn try_from(node_data: NodeData) -> Result<Self, Self::Error> {
        let addr: SocketAddr = node_data.address.parse().unwrap();
        let address: NodeAddressBytes = NymNodeRoutingAddress::from(addr).try_into().unwrap();

        // this has to be temporarily moved out of separate function as we can't return private types
        let pub_key = {
            let src = MixIdentityPublicKey::from_base58_string(node_data.public_key).to_bytes();
            let mut dest: [u8; 32] = [0; 32];
            dest.copy_from_slice(&src);
            nymsphinx::key::new(dest)
        };

        Ok(Node { address, pub_key })
    }
}

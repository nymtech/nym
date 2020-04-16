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
use curve25519_dalek::montgomery::MontgomeryPoint;
use nymsphinx::addressing::nodes::NymNodeRoutingAddress;
use serde::{Deserialize, Serialize};
use serde_json;
use sphinx::header::delays;
use sphinx::route::DestinationAddressBytes;
use sphinx::route::{Destination, Node};
use sphinx::SphinxPacket;
use std::convert::TryInto;
use std::time::Duration;

use wasm_bindgen::prelude::*;

mod utils;

const IDENTIFIER_LENGTH: usize = 16;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[derive(Serialize, Deserialize)]
pub struct Route {
    nodes: Vec<NodeData>,
}

#[wasm_bindgen]
#[derive(Serialize, Deserialize)]
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
pub fn create_sphinx_packet(rout: String, msg: &str, destination: &str) -> Vec<u8> {
    utils::set_panic_hook(); // nicer js errors.

    let r: Route = serde_json::from_str(&rout).unwrap();
    let mut route: Vec<Node> = vec![];
    for node in r.nodes.iter() {
        let address = node.address.parse().unwrap();
        let public_key = public_key_from_str(&node.public_key);
        let n = NymNodeRoutingAddress(address);
        let x = Node::new(n.try_into().unwrap(), public_key);
        route.push(x);
    }

    let average_delay = Duration::from_secs_f64(1.0);
    let delays = delays::generate_from_average_duration(route.len(), average_delay);
    let dest_bytes = DestinationAddressBytes::from_base58_string(destination.to_owned());
    let dest = Destination::new(dest_bytes, [4u8; IDENTIFIER_LENGTH]);

    let mut message = nymsphinx::chunking::split_and_prepare_payloads(&msg.as_bytes());
    let sphinx_packet =
        match SphinxPacket::new(message.pop().unwrap(), &route, &dest, &delays).unwrap() {
            SphinxPacket { header, payload } => SphinxPacket { header, payload },
        };

    sphinx_packet.to_bytes()
}

fn public_key_from_str(s: &str) -> MontgomeryPoint {
    let src = MixIdentityPublicKey::from_base58_string(s.to_owned()).to_bytes();
    let mut dest: [u8; 32] = [0; 32];
    dest.copy_from_slice(&src);
    MontgomeryPoint(dest)
}

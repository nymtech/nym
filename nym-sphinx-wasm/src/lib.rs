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
use sphinx::crypto;
use sphinx::header::delays;
use sphinx::route::{Destination, Node};
use sphinx::route::{DestinationAddressBytes, NodeAddressBytes};
use sphinx::SphinxPacket;
use std::time::Duration;

use wasm_bindgen::prelude::*;

mod utils;

const NODE_ADDRESS_LENGTH: usize = 32;
const DESTINATION_ADDRESS_LENGTH: usize = 32;
const IDENTIFIER_LENGTH: usize = 16;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

/// Creates a Sphinx packet for use in JavaScript applications, using wasm.
///
/// The `wasm-pack build` command will cause this to output JS bindings and a
/// wasm executable in the `pkg/` directory.
#[wasm_bindgen]
pub fn create_sphinx_packet() -> Vec<u8> {
    utils::set_panic_hook(); // nicer js errors.

    let (_, node1_pk) = crypto::keygen();
    let node1 = Node::new(
        NodeAddressBytes::from_bytes([5u8; NODE_ADDRESS_LENGTH]),
        node1_pk,
    );
    let (_, node2_pk) = crypto::keygen();
    let node2 = Node::new(
        NodeAddressBytes::from_bytes([4u8; NODE_ADDRESS_LENGTH]),
        node2_pk,
    );
    let (_, node3_pk) = crypto::keygen();
    let node3 = Node::new(
        NodeAddressBytes::from_bytes([2u8; NODE_ADDRESS_LENGTH]),
        node3_pk,
    );

    let route = [node1, node2, node3];
    let average_delay = Duration::from_secs_f64(1.0);
    let delays = delays::generate_from_average_duration(route.len(), average_delay);
    let destination = Destination::new(
        DestinationAddressBytes::from_bytes([3u8; DESTINATION_ADDRESS_LENGTH]),
        [4u8; IDENTIFIER_LENGTH],
    );

    let message = vec![13u8, 16];
    let sphinx_packet =
        match SphinxPacket::new(message.clone(), &route, &destination, &delays).unwrap() {
            SphinxPacket { header, payload } => SphinxPacket { header, payload },
        };

    sphinx_packet.to_bytes()
}

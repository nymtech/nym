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
use sphinx::header::delays;
use sphinx::route::{Destination, Node};
use sphinx::route::{DestinationAddressBytes, NodeAddressBytes};
use sphinx::SphinxPacket;
use std::convert::TryInto;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use wasm_bindgen::prelude::*;

mod utils;

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
    let address1 = NymNodeRoutingAddress(SocketAddr::new(IpAddr::from([216, 211, 110, 161]), 1789));
    let address_bytes1: NodeAddressBytes = address1.try_into().unwrap();
    let node1_pk = public_key_from_str("HShr7AQvJNxp64qKk23cmHUvnZqKD9BnpcUJWaskXiAN");
    let address2 = NymNodeRoutingAddress(SocketAddr::new(IpAddr::from([80, 218, 232, 173]), 1791));
    let address_bytes2: NodeAddressBytes = address2.try_into().unwrap();
    let node2_pk = public_key_from_str("8SMWvu6mMBwTNKyRcQx11J2mkZnYk5r6PJVKq2ZyT5P7");
    let address3 = NymNodeRoutingAddress(SocketAddr::new(IpAddr::from([213, 52, 129, 218]), 1789));
    let address_bytes3: NodeAddressBytes = address3.try_into().unwrap();
    let node3_pk = public_key_from_str("D4Zte8tBrc6aXW9BkG9rQNxG2PZ7Wzqk1sFCF6j4tiJN");
    let provider = NymNodeRoutingAddress(SocketAddr::new(IpAddr::from([139, 162, 246, 48]), 1789));
    let provider_bytes: NodeAddressBytes = provider.try_into().unwrap();
    let provider_pk = public_key_from_str("7vhgER4Gz789QHNTSu4apMpTcpTuUaRiLxJnbz1g2HFh");

    let node1 = Node::new(address_bytes1, node1_pk);
    let node2 = Node::new(address_bytes2, node2_pk);
    let node3 = Node::new(address_bytes3, node3_pk);
    let provider = Node::new(provider_bytes, provider_pk);

    let route = [node1, node2, node3, provider];
    let average_delay = Duration::from_secs_f64(1.0);
    let delays = delays::generate_from_average_duration(route.len(), average_delay);
    let destination = Destination::new(
        DestinationAddressBytes::from_bytes([3u8; DESTINATION_ADDRESS_LENGTH]),
        [4u8; IDENTIFIER_LENGTH],
    );

    let message = ("FOOMP FOOMP FOOMP\n\n").as_bytes().to_vec();
    let sphinx_packet =
        match SphinxPacket::new(message.clone(), &route, &destination, &delays).unwrap() {
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

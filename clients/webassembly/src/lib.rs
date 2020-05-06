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
use models::Topology;
use nymsphinx::addressing::nodes::NymNodeRoutingAddress;
use nymsphinx::Node as SphinxNode;
use nymsphinx::{delays, Destination, DestinationAddressBytes, NodeAddressBytes, SphinxPacket};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::convert::TryInto;
use std::net::SocketAddr;
use std::time::Duration;
use wasm_bindgen::prelude::*;

mod models;
mod utils;

pub use models::keys::keygen;

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

/// Creates a Gateway payload for use in JavaScript applications, using wasm.
/// It contains encoded address of first hop as well as the actual Sphinx Packet with the data.
///
/// The `wasm-pack build` command will cause this to output JS bindings and a
/// wasm executable in the `pkg/` directory.
///
/// Message chunking is currently not implemented. If the message exceeds the
/// capacity of a single Sphinx packet, the extra information will be discarded.
#[wasm_bindgen]
pub fn create_gateway_sphinx_packet(topology_json: &str, msg: &str, destination: &str) -> Vec<u8> {
    utils::set_panic_hook(); // nicer js errors.

    let route = sphinx_route_to(topology_json, destination);
    let average_delay = Duration::from_secs_f64(0.1);
    let delays = delays::generate_from_average_duration(route.len(), average_delay);
    let dest_bytes = DestinationAddressBytes::try_from_base58_string(destination).unwrap();
    let dest = Destination::new(dest_bytes, Default::default());

    // TODO: once we are able to reconstruct split messages use this instead
    // let split_message = split_and_prepare_payloads(&msg.as_bytes());
    // assert_eq!(split_message.len(), 1);
    // let message = split_message.first().unwrap().clone();

    let message = msg.as_bytes().to_vec();

    let sphinx_packet = SphinxPacket::new(message, &route, &dest, &delays, None).unwrap();
    gateway_payload(sphinx_packet, route)
}

/// Concatenate the first mix address bytes with the Sphinx packet.
///
/// The Nym gateway node has no idea what is inside the Sphinx packet, or where
/// it should send a packet it receives. So we prepend the packet with the
/// address bytes of the first mix inside the packet, so that the gateway can
/// forward the packet to it.
fn gateway_payload(sphinx_packet: SphinxPacket, route: Vec<SphinxNode>) -> Vec<u8> {
    let packet = sphinx_packet.to_bytes();
    let first_node_address =
        NymNodeRoutingAddress::try_from(route.first().unwrap().address.clone()).unwrap();

    first_node_address
        .as_bytes()
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
fn sphinx_route_to(topology_json: &str, recipient: &str) -> Vec<SphinxNode> {
    let topology = Topology::new(topology_json);
    let recipient_address = DestinationAddressBytes::try_from_base58_string(recipient).unwrap();
    let route = topology
        .random_route_to_client(recipient_address)
        .expect("invalid route produced - perhaps client has never registered?");
    assert_eq!(4, route.len());
    route
}

impl TryFrom<NodeData> for SphinxNode {
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

        Ok(SphinxNode { address, pub_key })
    }
}

#[cfg(test)]
mod test_constructing_a_sphinx_packet {
    use super::*;

    // the below test is no longer true, as the produced length is 1372 bytes + 7 (for IPV4) or + 19 (for IPV6)
    // conceptually everything works as before, only the 0 padding was removed as it served no purpose here

    // #[test]
    // fn produces_1404_bytes() {
    //     // 32 byte address + 1372 byte sphinx packet
    //     let packet = create_gateway_sphinx_packet(
    //         topology_fixture(),
    //         "foomp",
    //         "AetTDvynUNB2N35rvCVDxkPR593Cx4PCe4QQKrMgm5RR",
    //     );
    //     assert_eq!(1404, packet.len());
    // }

    #[test]
    fn starts_with_a_mix_address() {
        let mut payload = create_gateway_sphinx_packet(
            topology_fixture(),
            "foomp",
            "5pgrc4gPHP2tBQgfezcdJ2ZAjipoAsy6evrqHdxBbVXq",
        );
        // you don't really need 32 bytes here, but giving too much won't make it fail
        let mut address_buffer = [0; 32];
        let _ = payload.split_off(32);
        address_buffer.copy_from_slice(payload.as_slice());
        let address = NymNodeRoutingAddress::try_from_bytes(&address_buffer);

        assert!(address.is_ok());
    }
}

#[cfg(test)]
mod building_a_topology_from_json {
    use super::*;

    #[test]
    #[should_panic]
    fn panics_on_empty_string() {
        sphinx_route_to("", "5pgrc4gPHP2tBQgfezcdJ2ZAjipoAsy6evrqHdxBbVXq");
    }

    #[test]
    #[should_panic]
    fn panics_on_bad_json() {
        sphinx_route_to(
            "bad bad bad not json",
            "5pgrc4gPHP2tBQgfezcdJ2ZAjipoAsy6evrqHdxBbVXq",
        );
    }

    #[test]
    #[should_panic]
    fn panics_when_there_are_no_mixnodes() {
        let mut topology: Topology = serde_json::from_str(topology_fixture()).unwrap();
        topology.mix_nodes = vec![];
        let json = serde_json::to_string(&topology).unwrap();
        sphinx_route_to(&json, "5pgrc4gPHP2tBQgfezcdJ2ZAjipoAsy6evrqHdxBbVXq");
    }

    #[test]
    #[should_panic]
    fn panics_when_there_are_not_enough_mixnodes() {
        let mut topology: Topology = serde_json::from_str(topology_fixture()).unwrap();
        let node = topology.mix_nodes.first().unwrap().clone();
        topology.mix_nodes = vec![node]; // 1 mixnode isn't enough. Panic!
        let json = serde_json::to_string(&topology).unwrap();
        sphinx_route_to(&json, "5pgrc4gPHP2tBQgfezcdJ2ZAjipoAsy6evrqHdxBbVXq");
    }

    #[test]
    fn test_works_on_happy_json() {
        let route = sphinx_route_to(
            topology_fixture(),
            "5pgrc4gPHP2tBQgfezcdJ2ZAjipoAsy6evrqHdxBbVXq",
        );
        assert_eq!(4, route.len());
    }
}

#[cfg(test)]
fn topology_fixture() -> &'static str {
    r#"
        {
        "cocoNodes": [],
        "mixNodes": [
            {
            "host": "nym.300baud.de:1789",
            "pubKey": "AetTDvynUNB2N35rvCVDxkPR593Cx4PCe4QQKrMgm5RR",
            "version": "0.6.0",
            "location": "Falkenstein, DE",
            "layer": 3,
            "lastSeen": 1587572945877713700
            },
            {
            "host": "testnet_nymmixnode.roussel-zeter.eu:1789",
            "pubKey": "9wJ3zLoyat41e4ZgT1AWeueExv5c6uwnjvkRepj8Ebis",
            "version": "0.6.0",
            "location": "Geneva, CH",
            "layer": 3,
            "lastSeen": 1587572945907250400
            },
            {
            "host": "185.144.83.134:1789",
            "pubKey": "59tCzpCYsiKXz89rtvNiEYwQDdkseSShPEkifQXhsCgA",
            "version": "0.6.0",
            "location": "Bucharest",
            "layer": 1,
            "lastSeen": 1587572946007431400
            },
            {
            "host": "[2a0a:e5c0:2:2:0:c8ff:fe68:bf6b]:1789",
            "pubKey": "J9f9uS1hN8iwcN2STqH55fPRYqt7McEPyhNzpTYsxNdG",
            "version": "0.6.0",
            "location": "Glarus",
            "layer": 1,
            "lastSeen": 1587572945920982000
            },
            {
            "host": "[2a0a:e5c0:2:2:0:c8ff:fe68:bf6b]:1789",
            "pubKey": "J9f9uS1hN8iwcN2STqH55fPRYqt7McEPyhNzpTYsxNdG",
            "version": "0.6.0",
            "location": "Glarus",
            "layer": 2,
            "lastSeen": 1587572945920982000
            },
            {
            "host": "[2a0a:e5c0:2:2:0:c8ff:fe68:bf6b]:1789",
            "pubKey": "J9f9uS1hN8iwcN2STqH55fPRYqt7McEPyhNzpTYsxNdG",
            "version": "0.6.0",
            "location": "Glarus",
            "layer": 2,
            "lastSeen": 1587572945920982000
            }
        ],
        "mixProviderNodes":[],
        "gatewayNodes": [
            {
            "clientListener": "139.162.246.48:9000",
            "mixnetListener": "139.162.246.48:1789",
            "pubKey": "7vhgER4Gz789QHNTSu4apMpTcpTuUaRiLxJnbz1g2HFh",
            "version": "0.6.0",
            "location": "London, UK",
            "registeredClients": [
                {
                "pubKey": "5pgrc4gPHP2tBQgfezcdJ2ZAjipoAsy6evrqHdxBbVXq"
                }
            ],
            "lastSeen": 1587572946261865200
            },
            {
            "clientListener": "127.0.0.1:9000",
            "mixnetListener": "127.0.0.1:1789",
            "pubKey": "2XK8RDcUTRcJLUWoDfoXc2uP4YViscMLEM5NSzhSi87M",
            "version": "0.6.0",
            "location": "unknown",
            "registeredClients": [],
            "lastSeen": 1587572946304564700
            }
        ]
        }
    "#
}

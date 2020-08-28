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

use crate::models::client::NymClient;
use crate::utils::sleep;
use crate::websocket::JSWebsocket;
use crypto::asymmetric::{encryption, identity};
use futures::{SinkExt, StreamExt};
use gateway_requests::registration::handshake::client_handshake;
use js_sys::Promise;
pub use models::keys::keygen;
use rand::rngs::OsRng;
use tungstenite::{Error as WsError, Message as WsMessage};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::window;

mod models;
mod utils;
mod websocket;

// will cause messages to be written as if console.log("...") was called
#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => ($crate::log(&format_args!($($t)*).to_string()))
}

// will cause messages to be written as if console.warm("...") was called
#[macro_export]
macro_rules! console_warn {
    ($($t:tt)*) => ($crate::warn(&format_args!($($t)*).to_string()))
}

// will cause messages to be written as if console.error("...") was called
#[macro_export]
macro_rules! console_error {
    ($($t:tt)*) => ($crate::error(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    pub fn warn(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    pub fn error(s: &str);
}

pub const DEFAULT_RNG: OsRng = OsRng;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub async fn foomp() {
    utils::set_panic_hook();
    // let client = ClientTest {
    //             version: "0.8".to_string(),
    //             directory_server: "http://localhost:8080".to_string(),
    //     gateway_socket: JSWebsocket::new("ws://127.0.0.1:4000").unwrap(),
    //
    // };
    let version = "0.8.0-dev".to_string();
    let directory_server = "http://localhost:8080".to_string();

    let mut client = NymClient::new(directory_server, version);
    client = client.initial_setup().await;

    let active_native_client = "7yeAtiVGZFz5obya5uJ9ptBjFjkKKRPxaVBgRp33DkMz.CuWpunEFNo424vkEVQDwt45p91xX5JrnKc1htGSF6Wz@DicDxduuh3bKzNDHohikWXEkgqbzBj61EARPreShYK3f".to_string();
    let message = "hello from wasm!".to_string();

    client = client.send_message(message, active_native_client).await;
    // client = client.get_and_update_topology().await;
    // let gateway = client.choose_gateway();
    //
    // let gateway_id = gateway.identity_key.clone();
    // let gateway_address = gateway.client_listener.clone();
    //
    // client.connect_to_gateway(&gateway_address);

    // let topology = client.get_nym_topology().await;
    // console_log!("topology: {:#?}", topology);
    //
    // console_log!("foomp was called!");
    // let mut rng = OsRng;
    //
    // let mut socket = JSWebsocket::new("ws://127.0.0.1:4000").unwrap();
    //
    // let identity = identity::KeyPair::new_with_rng(&mut rng);
    // let gateway_pubkey =
    //     identity::PublicKey::from_base58_string("9Ku6ERQV6pmzTiwzZz5ffSazNyu68TtVTZ4n4Dih66cX")
    //         .unwrap();
    //
    // let shared_keys = client_handshake(&mut rng, &mut socket, &identity, gateway_pubkey).await;
    //
    // console_log!("got shared key! {:?}", shared_keys);

    // sleep(100).await.unwrap();
    //
    // let (mut sink, mut stream) = socket.split();
    //
    // spawn_local(async move {
    //     sink.send(WsMessage::Text("foomp1".into())).await.unwrap();
    //     sink.send(WsMessage::Text("foomp2".into())).await.unwrap();
    //     sink.send(WsMessage::Text("foomp3".into())).await.unwrap();
    // });
    //
    // spawn_local(async move {
    //     while let Some(received) = stream.next().await {
    //         console_log!("received {} from the socket!", received);
    //     }
    //     console_log!("won't get anything more")
    // });
    // // just for test to not have to bother with setting it all up
    //
    console_log!("waiting");
    sleep(10000).await.unwrap();
    //
    // // let topology = ClientTest::do_foomp().await;
    // //
    // // console_log!("{}", topology);
    // //
    // // // spawn_local(async {
    // // //     for i in 0..100 {
    // // //         console_log!("foomp {}", i);
    // // //         sleep(50).await.unwrap();
    // // //     }
    // // // });

    console_log!("foomp is done");
}

// /// Creates a Gateway payload for use in JavaScript applications, using wasm.
// /// It contains encoded address of first hop as well as the actual Sphinx Packet with the data.
// ///
// /// The `wasm-pack build` command will cause this to output JS bindings and a
// /// wasm executable in the `pkg/` directory.
// ///
// /// Message chunking is currently not implemented. If the message exceeds the
// /// capacity of a single Sphinx packet, the extra information will be discarded.
// ///
// #[wasm_bindgen]
// pub fn create_sphinx_packet(topology_json: &str, msg: &str, recipient: &str) -> Vec<u8> {
//     utils::set_panic_hook(); // nicer js errors.
//
//     let recipient = Recipient::try_from_base58_string(recipient).unwrap();
//
//     let route =
//         sphinx_route_to(topology_json, &recipient.gateway()).expect("todo: error handling...");
//     let average_delay = Duration::from_secs_f64(0.1);
//     let delays = delays::generate_from_average_duration(route.len(), average_delay);
//
//     // TODO: once we are able to reconstruct split messages use this instead
//     // let split_message = split_and_prepare_payloads(&msg.as_bytes());
//     // assert_eq!(split_message.len(), 1);
//     // let message = split_message.first().unwrap().clone();
//
//     let message = msg.as_bytes().to_vec();
//
//     let destination = recipient.as_sphinx_destination();
//     let sphinx_packet = SphinxPacket::new(message, &route, &destination, &delays).unwrap();
//     payload(sphinx_packet, route)
// }
//
// /// Concatenate the gateway address bytes with the Sphinx packet.
// ///
// /// The Nym gateway node has no idea what is inside the Sphinx packet, or where
// /// it should send a packet it receives. So we prepend the packet with the
// /// address bytes of the first mix inside the packet, so that the gateway can
// /// forward the packet to it.
// fn payload(sphinx_packet: SphinxPacket, route: Vec<SphinxNode>) -> Vec<u8> {
//     let packet = sphinx_packet.to_bytes();
//     let first_node_address =
//         NymNodeRoutingAddress::try_from(route.first().unwrap().address.clone()).unwrap();
//
//     first_node_address
//         .as_bytes()
//         .into_iter()
//         .chain(packet.into_iter())
//         .collect()
// }
//
// /// Attempts to create a Sphinx route, which is a `Vec<sphinx::Node>`, from a
// /// JSON string.
// ///
// /// # Panics
// ///
// /// This function panics if the supplied `raw_route` json string can't be
// /// extracted to a `JsonRoute`.
// fn sphinx_route_to(
//     topology_json: &str,
//     gateway_identity: &NodeIdentity,
// ) -> Option<Vec<SphinxNode>> {
//     let topology = Topology::new(topology_json);
//     let nym_topology: NymTopology = topology.try_into().ok()?;
//     let route = nym_topology
//         .random_route_to_gateway(&mut DEFAULT_RNG, DEFAULT_NUM_MIX_HOPS, gateway_identity)
//         .expect("invalid route produced");
//     assert_eq!(4, route.len());
//     Some(route)
// }
//
// impl TryFrom<NodeData> for SphinxNode {
//     // We really should start actually using errors rather than unwrapping on everything
//     type Error = ();
//
//     fn try_from(node_data: NodeData) -> Result<Self, Self::Error> {
//         let addr: SocketAddr = node_data.address.parse().unwrap();
//         let address: NodeAddressBytes = NymNodeRoutingAddress::from(addr).try_into().unwrap();
//         let pub_key = encryption::PublicKey::from_base58_string(node_data.public_key)
//             .unwrap()
//             .into();
//
//         Ok(SphinxNode { address, pub_key })
//     }
// }

// Copyright 2018 Parity Technologies (UK) Ltd.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

//! A basic chat application with logs demonstrating libp2p and the gossipsub protocol
//! combined with mDNS for the discovery of peers to gossip with.
//!
//! Using two terminal windows, start two instances, typing the following in each:
//!
//! ```sh
//! cargo run
//! ```
//!
//! Mutual mDNS discovery may take a few seconds. When each peer does discover the other
//! it will print a message like:
//!
//! ```sh
//! mDNS discovered a new peer: {peerId}
//! ```
//!
//! Type a message and hit return: the message is sent and printed in the other terminal.
//! Close with Ctrl-c.
//!
//! You can open more terminal windows and add more peers using the same line above.
//!
//! Once an additional peer is mDNS discovered it can participate in the conversation
//! and all peers will receive messages sent from it.
//!
//! If a participant exits (Control-C or otherwise) the other peers will receive an mDNS expired
//! event and remove the expired peer from the list of known peers.

// use crate::rust_libp2p_nym::transport::NymTransport;
// use futures::{prelude::*, select};
// use libp2p::Multiaddr;
// use libp2p::{
//     core::muxing::StreamMuxerBox,
//     gossipsub, identity,
//     swarm::NetworkBehaviour,
//     swarm::{SwarmBuilder, SwarmEvent},
//     PeerId, Transport,
// };
// use log::{error, info, LevelFilter};
// use nym_sdk::mixnet::MixnetClient;
// use std::collections::hash_map::DefaultHasher;
use std::error::Error;
// use std::hash::{Hash, Hasher};
// use std::time::Duration;
// use tokio::io;
// use tokio_util::codec;

// #[path = "../libp2p_shared/lib.rs"]
// mod rust_libp2p_nym;
//
// // We create a custom network behaviour that uses Gossipsub
// #[derive(NetworkBehaviour)]
// struct Behaviour {
//     gossipsub: gossipsub::Behaviour,
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    unimplemented!("temporarily disabled")

    // pretty_env_logger::formatted_timed_builder()
    //     .filter_level(LevelFilter::Warn)
    //     .filter(Some("libp2p_chat"), LevelFilter::Info)
    //     .init();
    //
    // // Create a random PeerId
    // let id_keys = identity::Keypair::generate_ed25519();
    // let local_peer_id = PeerId::from(id_keys.public());
    // info!("Local peer id: {local_peer_id}");
    //
    // // To content-address message, we can take the hash of message and use it as an ID.
    // let message_id_fn = |message: &gossipsub::Message| {
    //     let mut s = DefaultHasher::new();
    //     message.data.hash(&mut s);
    //     gossipsub::MessageId::from(s.finish().to_string())
    // };
    //
    // // Set a custom gossipsub configuration
    // let gossipsub_config = gossipsub::ConfigBuilder::default()
    //     .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
    //     .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
    //     .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
    //     .build()
    //     .expect("Valid config");
    //
    // // build a gossipsub network behaviour
    // let mut gossipsub = gossipsub::Behaviour::new(
    //     gossipsub::MessageAuthenticity::Signed(id_keys),
    //     gossipsub_config,
    // )
    // .expect("Correct configuration");
    // // Create a Gossipsub topic
    // let topic = gossipsub::IdentTopic::new("test-net");
    // // subscribes to our topic
    // gossipsub.subscribe(&topic)?;
    //
    // let client = MixnetClient::connect_new().await.unwrap();
    // info!("client address: {}", client.nym_address());
    //
    // let local_key = identity::Keypair::generate_ed25519();
    // let local_peer_id = PeerId::from(local_key.public());
    // info!("Local peer id: {local_peer_id:?}");
    //
    // let transport = NymTransport::new(client, local_key).await?;
    //
    // let mut swarm = SwarmBuilder::with_tokio_executor(
    //     transport
    //         .map(|a, _| (a.0, StreamMuxerBox::new(a.1)))
    //         .boxed(),
    //     Behaviour { gossipsub },
    //     local_peer_id,
    // )
    // .build();
    //
    // if let Some(addr) = std::env::args().nth(1) {
    //     let remote: Multiaddr = addr.parse()?;
    //     swarm.dial(remote)?;
    //     info!("Dialed {addr}")
    // }
    //
    // // Read full lines from stdin
    // let mut stdin = codec::FramedRead::new(io::stdin(), codec::LinesCodec::new()).fuse();
    //
    // info!("Enter messages via STDIN and they will be sent to connected peers using Gossipsub");
    //
    // // Kick it off
    // loop {
    //     select! {
    //         line = stdin.select_next_some() => {
    //             if let Err(e) = swarm
    //                 .behaviour_mut().gossipsub
    //                 .publish(topic.clone(), line.expect("Stdin not to close").as_bytes()) {
    //                 error!("Publish error: {e:?}");
    //             }
    //         },
    //         event = swarm.select_next_some() => {
    //             match event {
    //                 SwarmEvent::Behaviour(BehaviourEvent::Gossipsub(gossipsub::Event::Message {
    //                     propagation_source: peer_id,
    //                     message_id: id,
    //                     message,
    //                 })) => info!(
    //                         "Got message: '{}' with id: {id} from peer: {peer_id}",
    //                         String::from_utf8_lossy(&message.data),
    //                     ),
    //                 SwarmEvent::NewListenAddr { address, .. } => {
    //                     info!("Local node is listening on {address}");
    //                 }
    //                 other => {info!("other event: {:?}", other)}
    //             }
    //         }
    //     }
    // }
}

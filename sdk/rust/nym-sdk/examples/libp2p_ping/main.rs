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

//! Ping example
//!
//! See ../src/tutorial.rs for a step-by-step guide building the example below.
//!
//! In the first terminal window, run:
//!
//! ```sh
//! cargo run --example ping --features=full
//! ```
//!
//! It will print the PeerId and the listening addresses, e.g. `Listening on
//! "/ip4/0.0.0.0/tcp/24915"`
//!
//! In the second terminal window, start a new instance of the example with:
//!
//! ```sh
//! cargo run --example ping --features=full -- /ip4/127.0.0.1/tcp/24915
//! ```
//!
//! The two nodes establish a connection, negotiate the ping protocol
//! and begin pinging each other.

// use libp2p::futures::StreamExt;
// use libp2p::ping::Success;
// use libp2p::swarm::{keep_alive, NetworkBehaviour, SwarmEvent};
// use libp2p::{identity, ping, Multiaddr, PeerId};
// use log::{debug, info, LevelFilter};
use std::error::Error;
// use std::time::Duration;
//
// #[path = "../libp2p_shared/lib.rs"]
// mod rust_libp2p_nym;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    unimplemented!("temporarily disabled")
    //
    // pretty_env_logger::formatted_timed_builder()
    //     .filter_level(LevelFilter::Warn)
    //     .filter(Some("libp2p_ping"), LevelFilter::Debug)
    //     .init();
    //
    // let local_key = identity::Keypair::generate_ed25519();
    // let local_peer_id = PeerId::from(local_key.public());
    // info!("Local peer id: {local_peer_id:?}");
    //
    // #[cfg(not(feature = "libp2p-vanilla"))]
    // let mut swarm = {
    //     debug!("Running `ping` example using NymTransport");
    //     use libp2p::core::{muxing::StreamMuxerBox, transport::Transport};
    //     use libp2p::swarm::SwarmBuilder;
    //     use rust_libp2p_nym::transport::NymTransport;
    //
    //     let client = nym_sdk::mixnet::MixnetClient::connect_new().await.unwrap();
    //
    //     let transport = NymTransport::new(client, local_key.clone()).await?;
    //     SwarmBuilder::with_tokio_executor(
    //         transport
    //             .map(|a, _| (a.0, StreamMuxerBox::new(a.1)))
    //             .boxed(),
    //         Behaviour::default(),
    //         local_peer_id,
    //     )
    //     .build()
    // };
    //
    // #[cfg(feature = "libp2p-vanilla")]
    // let mut swarm = {
    //     debug!("Running `ping` example using the vanilla libp2p tokio_development_transport");
    //     let transport = libp2p::tokio_development_transport(local_key)?;
    //     let mut swarm =
    //         libp2p::Swarm::with_tokio_executor(transport, Behaviour::default(), local_peer_id);
    //     swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    //     swarm
    // };
    //
    // // Dial the peer identified by the multi-address given as the second
    // // command-line argument, if any.
    // if let Some(addr) = std::env::args().nth(1) {
    //     let remote: Multiaddr = addr.parse()?;
    //     swarm.dial(remote)?;
    //     info!("Dialed {addr}")
    // }
    //
    // let mut total_ping_rtt: Duration = Duration::from_micros(0);
    // let mut counter: u128 = 0;
    // loop {
    //     match swarm.select_next_some().await {
    //         SwarmEvent::NewListenAddr { address, .. } => info!("Listening on {address:?}"),
    //         SwarmEvent::Behaviour(event) => {
    //             // Get the round-trip duration for the pings.
    //             // This value is already captured in the BehaviourEvent::Ping's `Success::Ping`
    //             // field.
    //             debug!("{event:?}");
    //             if let BehaviourEvent::Ping(ping_event) = event {
    //                 let result: Success = ping_event.result?;
    //                 match result {
    //                     Success::Ping { rtt } => {
    //                         counter += 1;
    //                         total_ping_rtt += rtt;
    //                         let average_ping_rtt = Duration::from_micros(
    //                             (total_ping_rtt.as_micros() / counter).try_into().unwrap(),
    //                         );
    //                         info!("Ping RTT: {rtt:?} AVERAGE RTT: ({counter} pings): {average_ping_rtt:?}");
    //                     }
    //                     Success::Pong => info!("Pong Event"),
    //                 }
    //             }
    //         }
    //         _ => {}
    //     }
    // }
}
//
// /// Our network behaviour.
// ///
// /// For illustrative purposes, this includes the [`KeepAlive`](behaviour::KeepAlive) behaviour so a continuous sequence of
// /// pings can be observed.
// #[derive(NetworkBehaviour, Default)]
// struct Behaviour {
//     keep_alive: keep_alive::Behaviour,
//     ping: ping::Behaviour,
// }

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

//! identify example
//!
//! In the first terminal window, run:
//!
//! ```sh
//! cargo run
//! ```
//! It will print the [`PeerId`] and the listening addresses, e.g. `Listening on
//! "/ip4/127.0.0.1/tcp/24915"`
//!
//! In the second terminal window, start a new instance of the example with:
//!
//! ```sh
//! cargo run -- /ip4/127.0.0.1/tcp/24915
//! ```
//! The two nodes establish a connection, negotiate the identify protocol
//! and will send each other identify info which is then printed to the console.

use crate::rust_libp2p_nym::transport::NymTransport;
use futures::prelude::*;
use libp2p::swarm::SwarmEvent::ListenerError;
use libp2p::swarm::{keep_alive, KeepAlive, NetworkBehaviour};
use libp2p::Multiaddr;
use libp2p::{identify, identity, ping, swarm::SwarmEvent, PeerId, Transport};
use log::{debug, LevelFilter, info};
use nym_sdk::mixnet::MixnetClient;
use std::error::Error;
use tokio::time::Duration;

#[path = "../libp2p_shared/lib.rs"]
mod rust_libp2p_nym;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::formatted_timed_builder()
        .filter_level(log::LevelFilter::Warn)
        .filter(Some("libp2p_identify"), LevelFilter::Debug)
        .init();

    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {local_peer_id:?}");

    // Create a identify network behaviour.
    // TODO bring in other behaviour attributes (Ping, KeepAlive)
    // let behaviour = identify::Behaviour::new(identify::Config::new(
    //     "/ipfs/id/2.0.0".to_string(),
    //     local_key.public(),
    // ));

    let client = MixnetClient::connect_new().await.unwrap();
    let transport = NymTransport::new(client, local_key.clone()).await?;
    let listen_addr = transport.listen_addr.clone();

    let mut swarm = {
        debug!("Running `identify` example using NymTransport");
        use libp2p::core::{muxing::StreamMuxerBox, transport::Transport};
        use libp2p::swarm::SwarmBuilder;

        SwarmBuilder::with_tokio_executor(
            transport
                .map(|a, _| (a.0, StreamMuxerBox::new(a.1)))
                .boxed(),
            // behaviour,
            MyBehaviour {
                identify: identify::Behaviour::new(identify::Config::new(
                    "/ipfs/id/2.0.0".to_string(),
                    local_key.public(),
                )),
                ping: ping::Behaviour::new(
                    ping::Config::new().with_interval(Duration::from_secs(1)),
                ),
                keep_alive: keep_alive::Behaviour,
            },
            local_peer_id,
        )
        .build()
    };

    let _ = swarm.listen_on(listen_addr.clone())?;

    // Dial the peer identified by the multi-address given as the second
    // command-line argument, if any.
    if let Some(addr) = std::env::args().nth(1) {
        let remote: Multiaddr = addr.parse()?;
        swarm.dial(remote)?;
        println!("Dialed {addr}")
    }

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => println!("Listening on {address:?}"),
            // Prints peer id identify info is being sent to.
            SwarmEvent::Behaviour(MyBehaviourEvent::Identify(identify::Event::Sent {
                peer_id,
                ..
            })) => {
                info!("Sent identify info to {peer_id:?}")
            }
            // Prints out the info received via the identify event
            SwarmEvent::Behaviour(MyBehaviourEvent::Identify(identify::Event::Received {
                info,
                ..
            })) => {
                info!("Received {info:?}")
            }
            // START DEBUG PRINTING TO FIND other
            SwarmEvent::Dialing(peer_id) => {
                info!("Dial attempt from {:?}", peer_id)
            }
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                info!(
                    "Connection closed with peer: {:?} because: {:?}",
                    peer_id, cause
                )
            }
            SwarmEvent::IncomingConnection {
                local_addr,
                send_back_addr,
            } => {
                info!("Incoming connection from: {:?}, with sendback address: {:?}", local_addr, send_back_addr)
            }
            SwarmEvent::IncomingConnectionError { error, .. } => {
                info!("Incoming connection error: {error:?}")
            }
            SwarmEvent::ExpiredListenAddr {
                listener_id,
                address,
            } => {
                info!("Expired listener {listener_id:?} {address:?}")
            }
            SwarmEvent::ListenerError { listener_id, error } => {
                info!("{listener_id:?} stopped listening with {error:?}")
            }
            // END DEBUG PRINTING
            _ => {
                // println!("got something else")
            }
        }
    }
}

/// Our network behaviour.
///
/// For illustrative purposes, this includes the [`KeepAlive`](behaviour::KeepAlive) behaviour so a continuous sequence of
/// pings can be observed.
#[derive(NetworkBehaviour)]
struct MyBehaviour {
    identify: identify::Behaviour,
    keep_alive: keep_alive::Behaviour,
    ping: ping::Behaviour,
}

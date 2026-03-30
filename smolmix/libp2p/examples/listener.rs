//! Clearnet libp2p listener that accepts pings.
//!
//! Run this first, then pass the printed address to the `ping` example which
//! dials through the Nym mixnet.
//!
//! Run with:
//!   cargo run -p smolmix-libp2p --example listener
//!
//! Then in another terminal:
//!   cargo run -p smolmix-libp2p --example ping -- <MULTIADDR from listener output>

use futures::StreamExt;
use libp2p::swarm::SwarmEvent;
use libp2p::{noise, ping, yamux, SwarmBuilder};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            Default::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|_| ping::Behaviour::default())?
        .build();

    let local_peer_id = *swarm.local_peer_id();
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    println!("Peer ID: {local_peer_id}");
    println!("Waiting for listen address...");

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                let full_addr = format!("{address}/p2p/{local_peer_id}");
                println!();
                println!("Listening! Pass this address to the ping example:");
                println!("  {full_addr}");
                println!();
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                println!("Connection established from {peer_id}");
            }
            SwarmEvent::Behaviour(ping::Event { peer, result, .. }) => match result {
                Ok(rtt) => println!("Ping from {peer}: {rtt:?}"),
                Err(e) => println!("Ping error from {peer}: {e}"),
            },
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                println!("Connection closed: {peer_id}");
            }
            _ => {}
        }
    }
}

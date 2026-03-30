//! libp2p ping through the Nym mixnet.
//!
//! Dials a libp2p peer through the mixnet and exchanges pings. Start the
//! `listener` example first to get an address, then pass it here.
//!
//! Run with:
//!   # Terminal 1: start the clearnet listener
//!   cargo run -p smolmix-libp2p --example listener
//!
//!   # Terminal 2: dial through the mixnet
//!   cargo run -p smolmix-libp2p --example ping -- <MULTIADDR from listener>
//!   cargo run -p smolmix-libp2p --example ping -- --ipr <IPR_ADDRESS> <MULTIADDR>

use futures::StreamExt;
use libp2p::core::upgrade::Version;
use libp2p::core::Transport;
use libp2p::swarm::SwarmEvent;
use libp2p::{noise, ping, yamux, Multiaddr, SwarmBuilder};
use smolmix::{Recipient, Tunnel};
use smolmix_libp2p::SmolmixTransport;
use tracing::info;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    smolmix::init_logging();

    let args: Vec<String> = std::env::args().collect();

    // Parse optional --ipr flag
    let ipr_pos = args.iter().position(|a| a == "--ipr");
    let ipr_addr = ipr_pos.and_then(|i| args.get(i + 1));

    // The multiaddr is the last positional argument (skip --ipr and its value)
    let multiaddr_str = args
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != 0 && Some(*i) != ipr_pos && Some(*i) != ipr_pos.map(|p| p + 1))
        .map(|(_, s)| s)
        .last()
        .expect(
            "Usage: ping [--ipr <IPR_ADDRESS>] <MULTIADDR>\n\
             \n\
             Start the listener first:\n\
             \x20 cargo run -p smolmix-libp2p --example listener",
        );
    let remote: Multiaddr = multiaddr_str.parse()?;

    // Create the tunnel
    let tunnel = if let Some(addr) = ipr_addr {
        let recipient: Recipient = addr.parse().expect("invalid IPR address");
        Tunnel::new_with_ipr(recipient).await?
    } else {
        Tunnel::new().await?
    };

    // Build the swarm with our mixnet transport
    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_other_transport(|keypair| {
            SmolmixTransport::new(&tunnel)
                .upgrade(Version::V1)
                .authenticate(noise::Config::new(keypair).expect("noise config"))
                .multiplex(yamux::Config::default())
                .boxed()
        })?
        .with_behaviour(|_| ping::Behaviour::default())?
        .build();

    info!("Local peer ID: {}", swarm.local_peer_id());
    info!("Dialing {remote} through the Nym mixnet...");
    swarm.dial(remote)?;

    // Drive the swarm and print ping results
    let mut pings = 0u32;
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::Behaviour(ping::Event { peer, result, .. }) => match result {
                Ok(rtt) => {
                    pings += 1;
                    info!("Ping #{pings} to {peer}: {rtt:?}");
                    if pings >= 5 {
                        info!("Done — {pings} pings completed.");
                        break;
                    }
                }
                Err(e) => {
                    info!("Ping error from {peer}: {e}");
                    break;
                }
            },
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                info!("Connected to {peer_id}");
            }
            SwarmEvent::OutgoingConnectionError { error, .. } => {
                info!("Connection error: {error}");
                break;
            }
            _ => {}
        }
    }

    tunnel.shutdown().await;
    Ok(())
}

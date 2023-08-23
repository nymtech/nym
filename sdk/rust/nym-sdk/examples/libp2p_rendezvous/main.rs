
use futures::StreamExt;
use libp2p::{
    core::transport::upgrade::Version,
    identify, identity, noise, ping, rendezvous,
    tcp, yamux, PeerId, Transport,
};
use libp2p::swarm::{keep_alive, NetworkBehaviour, SwarmBuilder, SwarmEvent}; 
use std::time::Duration;
use log::{debug, info, LevelFilter};
use nym_sdk::mixnet::MixnetClient;
use std::error::Error; 

#[path = "../libp2p_shared/lib.rs"]
mod rust_libp2p_nym;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::formatted_timed_builder()
        .filter_level(LevelFilter::Warn)
        .filter(Some("libp2p_rendezvous"), LevelFilter::Debug)
        .init();

    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    info!("Local peer id: {local_peer_id:?}"); 

    // let mut swarm = SwarmBuilder::with_tokio_executor(
    //     tcp::tokio::Transport::default()
    //         .upgrade(Version::V1Lazy)
    //         .authenticate(noise::Config::new(&key_pair).unwrap())
    //         .multiplex(yamux::Config::default())
    //         .boxed(),
    //     MyBehaviour {
    //         identify: identify::Behaviour::new(identify::Config::new(
    //             "rendezvous-example/1.0.0".to_string(),
    //             key_pair.public(),
    //         )),
    //         rendezvous: rendezvous::server::Behaviour::new(rendezvous::server::Config::default()),
    //         ping: ping::Behaviour::new(ping::Config::new().with_interval(Duration::from_secs(1))),
    //         keep_alive: keep_alive::Behaviour,
    //     },
    //     PeerId::from(key_pair.public()),
    // )
    // .build();

    let mut swarm = {
        debug!("Running `rendezvous server` example using NymTransport");
        use libp2p::core::{muxing::StreamMuxerBox, transport::Transport};
        use libp2p::swarm::SwarmBuilder;
        use rust_libp2p_nym::transport::NymTransport;

        let client = MixnetClient::connect_new().await.unwrap();

        let transport = NymTransport::new(client, local_key.clone()).await?;
        SwarmBuilder::with_tokio_executor(
            transport
                .map(|a, _| (a.0, StreamMuxerBox::new(a.1)))
                .boxed(),
            MyBehaviour {
            identify: identify::Behaviour::new(identify::Config::new(
                "rendezvous-example/1.0.0".to_string(),
                local_key.public(),
            )),
            rendezvous: rendezvous::server::Behaviour::new(rendezvous::server::Config::default()),
            ping: ping::Behaviour::new(ping::Config::new().with_interval(Duration::from_secs(1))),
            keep_alive: keep_alive::Behaviour,
            },
            local_peer_id,
        )
        .build()
    };

    log::info!("Local peer id: {}", swarm.local_peer_id());

    // let _ = swarm.listen_on("/ip4/0.0.0.0/tcp/62649".parse().unwrap());

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => info!("Listening on {address:?}"),
            SwarmEvent::Behaviour(event) => {
                debug!("{event:?}");
            }
            other => {
                log::debug!("Unhandled {:?}", other);
            }
        }
    }

    // while let Some(event) = swarm.next().await {
    //     match event {
    //         SwarmEvent::ConnectionEstablished { peer_id, .. } => {
    //             log::info!("Connected to {}", peer_id);
    //         }
    //         SwarmEvent::ConnectionClosed { peer_id, .. } => {
    //             log::info!("Disconnected from {}", peer_id);
    //         }
    //         SwarmEvent::Behaviour(MyBehaviourEvent::Rendezvous(
    //             rendezvous::server::Event::PeerRegistered { peer, registration },
    //         )) => {
    //             log::info!(
    //                 "Peer {} registered for namespace '{}'",
    //                 peer,
    //                 registration.namespace
    //             );
    //         }
    //         SwarmEvent::Behaviour(MyBehaviourEvent::Rendezvous(
    //             rendezvous::server::Event::DiscoverServed {
    //                 enquirer,
    //                 registrations,
    //             },
    //         )) => {
    //             log::info!(
    //                 "Served peer {} with {} registrations",
    //                 enquirer,
    //                 registrations.len()
    //             );
    //         }
    //         // add a nym-specific newlisten behaviour? 
    //         other => {
    //             log::debug!("Unhandled {:?}", other);
    //         }
    //     }
    // }

    Ok(()) 
}

#[derive(NetworkBehaviour)]
struct MyBehaviour {
    identify: identify::Behaviour,
    rendezvous: rendezvous::server::Behaviour,
    ping: ping::Behaviour,
    keep_alive: keep_alive::Behaviour,
}

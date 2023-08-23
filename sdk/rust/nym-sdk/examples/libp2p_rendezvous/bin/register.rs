use libp2p::futures::StreamExt;
use libp2p::ping::Success;
use libp2p::swarm::{keep_alive, NetworkBehaviour, SwarmEvent};
use libp2p::{identity, ping, Multiaddr, PeerId};
use log::{debug, info, LevelFilter};
use nym_sdk::mixnet::MixnetClient;
use std::error::Error;
use std::time::Duration;

#[tokio::main]
async fn main() {
    pretty_env_logger::formatted_timed_builder()
        .filter_level(LevelFilter::Warn)
        .filter(Some("libp2p_rendezvous"), LevelFilter::Debug)
        .init();

    let key_pair = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    info!("Local peer id: {local_peer_id:?}"); 

    // copy and paste in from the cli for the moment 
    // TODO take from cli as args 
    let rendezvous_point_address = "/nym/6a9Lxos7r5oh1jHNUDHdXW6NjoAiMk3fvSUYeCdP7p6f.6scJ3WbQKHw6m1vowTHedonGzunDXABnDjLV5Jgg95UT@FyHgeVWeXTysBd7515ndZ2tpzWhv9myLfuv4S9oeoFpR".parse::<Multiaddr>().unwrap();
    let rendezvous_point = "12D3KooWJFjuHa68RpcwgRFLYwEiT6TcwWRneEgzSExP5dQ81Yrk"
        .parse()
        .unwrap();

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
                rendezvous: rendezvous::server::Behaviour::new(rendezvous::server::Config::default()),
                ping: ping::Behaviour::new(ping::Config::new().with_interval(Duration::from_secs(1))),
                keep_alive: keep_alive::Behaviour,
            },
            local_peer_id,
        )
        .build()
    };

    log::info!("Local peer id: {}", swarm.local_peer_id());

    // In production the external address should be the publicly facing IP address of the rendezvous point.
    // This address is recorded in the registration entry by the rendezvous point.
    /* 
    TODO we need the nym multiaddr of this instance
    */
    let external_address = "/ip4/127.0.0.1/tcp/0".parse::<Multiaddr>().unwrap();
    swarm.add_external_address(external_address);

    log::info!("Local peer id: {}", swarm.local_peer_id());

    swarm.dial(rendezvous_point_address.clone()).unwrap();

    match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                log::info!("Listening on {}", address);
            }
            SwarmEvent::ConnectionClosed {
                peer_id,
                cause: Some(error),
                ..
            } if peer_id == rendezvous_point => {
                log::error!("Lost connection to rendezvous point {}", error);
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } if peer_id == rendezvous_point => {
                if let Err(error) = swarm.behaviour_mut().rendezvous.register(
                    rendezvous::Namespace::from_static("rendezvous"),
                    rendezvous_point,
                    None,
                ) {
                    log::error!("Failed to register: {error}");
                    return;
                }
                log::info!("Connection established with rendezvous point {}", peer_id);
            }
            // once `/identify` did its job, we know our external address and can register
            SwarmEvent::Behaviour(MyBehaviourEvent::Rendezvous(
                rendezvous::client::Event::Registered {
                    namespace,
                    ttl,
                    rendezvous_node,
                },
            )) => {
                log::info!(
                    "Registered for namespace '{}' at rendezvous point {} for the next {} seconds",
                    namespace,
                    rendezvous_node,
                    ttl
                );
            }
            SwarmEvent::Behaviour(MyBehaviourEvent::Rendezvous(
                rendezvous::client::Event::RegisterFailed {
                    rendezvous_node,
                    namespace,
                    error,
                },
            )) => {
                log::error!(
                    "Failed to register: rendezvous_node={}, namespace={}, error_code={:?}",
                    rendezvous_node,
                    namespace,
                    error
                );
                return;
            }
            SwarmEvent::Behaviour(MyBehaviourEvent::Ping(ping::Event {
                peer,
                result: Ok(rtt),
                ..
            })) if peer != rendezvous_point => {
                log::info!("Ping to {} is {}ms", peer, rtt.as_millis())
            }
            other => {
                log::debug!("Unhandled {:?}", other);
            }
    }
}

#[derive(NetworkBehaviour)]
struct MyBehaviour {
    rendezvous: rendezvous::client::Behaviour,
    ping: ping::Behaviour,
    keep_alive: keep_alive::Behaviour,
}

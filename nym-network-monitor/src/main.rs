use crate::http::HttpServer;
use anyhow::Result;
use clap::Parser;
use log::{info, warn};
use nym_network_defaults::setup_env;
use nym_sdk::mixnet::{self, MixnetClient};
use nym_topology::{HardcodedTopologyProvider, NymTopology};
use std::{
    collections::VecDeque,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::{signal::ctrl_c, sync::RwLock};
use tokio_util::sync::CancellationToken;

mod accounting;
mod handlers;
mod http;

/// Simple program to greet a person
pub type ClientsWrapper = Arc<RwLock<VecDeque<Arc<RwLock<MixnetClient>>>>>;

async fn make_clients(
    clients: ClientsWrapper,
    n_clients: usize,
    lifetime: u64,
    topology: NymTopology,
) {
    loop {
        let spawned_clients = clients.read().await.len();
        info!("Currently spawned clients: {}", spawned_clients);
        // If we have enough clients, sleep for a minute and remove the oldest one
        if spawned_clients >= n_clients {
            info!("New client will be spawned in 1 minute");
            tokio::time::sleep(tokio::time::Duration::from_secs(lifetime)).await;
            info!("Removing oldest client");
            let dropped_client = clients.write().await.pop_front().unwrap();
            loop {
                if Arc::strong_count(&dropped_client) == 1 {
                    let client = Arc::into_inner(dropped_client).unwrap().into_inner();
                    client.disconnect().await;
                    break;
                }
                info!("Client still in use, waiting 2 seconds");
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
        info!("Spawning new client");
        let client = match make_client(topology.clone()).await {
            Ok(client) => client,
            Err(err) => {
                warn!("{}, moving on", err);
                continue;
            }
        };
        clients
            .write()
            .await
            .push_back(Arc::new(RwLock::new(client)));
    }
}

async fn make_client(topology: NymTopology) -> Result<MixnetClient> {
    let net = mixnet::NymNetworkDetails::new_from_env();
    let topology_provider = Box::new(HardcodedTopologyProvider::new(topology));
    let mixnet_client = mixnet::MixnetClientBuilder::new_ephemeral()
        .network_details(net)
        .custom_topology_provider(topology_provider)
        // .enable_credentials_mode()
        .build()?;

    let client = mixnet_client.connect_to_mixnet().await?;
    Ok(client)
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Number of clients to spawn
    #[arg(short = 'C', long = "clients", default_value_t = 10)]
    n_clients: usize,

    /// Lifetime of each client in seconds
    #[arg(short = 'T', long, default_value_t = 60)]
    client_lifetime: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    nym_bin_common::logging::setup_logging();

    let args = Args::parse();

    setup_env(Some("../envs/mainnet.env"));

    let cancel_token = CancellationToken::new();
    let server_cancel_token = cancel_token.clone();
    let clients = Arc::new(RwLock::new(VecDeque::with_capacity(args.n_clients)));
    let topology = NymTopology::new_from_file("topology.json").unwrap();

    let spawn_clients = Arc::clone(&clients);
    tokio::spawn(make_clients(
        spawn_clients,
        args.n_clients,
        args.client_lifetime,
        topology,
    ));

    let _server_handle = tokio::spawn(async move {
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let server = HttpServer::new(socket, server_cancel_token);
        server.run(clients).await
    });

    ctrl_c().await?;
    println!("received ctrl-c");

    cancel_token.cancel();

    println!("Waiting for message (ctrl-c to exit)");

    Ok(())
}

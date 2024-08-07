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
    str::FromStr,
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
            info!("New client will be spawned in {} seconds", lifetime);
            tokio::time::sleep(tokio::time::Duration::from_secs(lifetime)).await;
            info!("Removing oldest client");
            if let Some(dropped_client) = clients.write().await.pop_front() {
                loop {
                    if Arc::strong_count(&dropped_client) == 1 {
                        if let Some(client) = Arc::into_inner(dropped_client) {
                            client.into_inner().disconnect().await;
                        } else {
                            warn!("Failed to drop client, client had more then one strong ref")
                        }
                        break;
                    }
                    info!("Client still in use, waiting 2 seconds");
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
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

    /// Port to listen on
    #[arg(long, default_value_t = 8080)]
    port: u16,

    /// Host to listen on
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Path to the topology file
    #[arg(short, long, default_value = None)]
    topology: Option<String>,

    /// Path to the environment file
    #[arg(short, long, default_value = None)]
    env: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    nym_bin_common::logging::setup_logging();

    let args = Args::parse();

    setup_env(args.env);

    let cancel_token = CancellationToken::new();
    let server_cancel_token = cancel_token.clone();
    let clients = Arc::new(RwLock::new(VecDeque::with_capacity(args.n_clients)));

    let topology = if let Some(topology_file) = args.topology {
        NymTopology::new_from_file(topology_file)?
    } else {
        NymTopology::new_from_env().await?
    };

    let spawn_clients = Arc::clone(&clients);
    tokio::spawn(make_clients(
        spawn_clients,
        args.n_clients,
        args.client_lifetime,
        topology,
    ));

    let _server_handle = tokio::spawn(async move {
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::from_str(&args.host)?), args.port);
        let server = HttpServer::new(socket, server_cancel_token);
        server.run(clients).await
    });

    info!("Waiting for message (ctrl-c to exit)");

    ctrl_c().await?;
    info!("Received Ctrl-C");

    cancel_token.cancel();
    Ok(())
}

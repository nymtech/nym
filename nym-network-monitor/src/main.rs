use crate::http::HttpServer;
use accounting::submit_metrics;
use anyhow::Result;
use clap::Parser;
use log::{info, warn};
use nym_crypto::asymmetric::ed25519::PrivateKey;
use nym_network_defaults::setup_env;
use nym_network_defaults::var_names::NYM_API;
use nym_sdk::mixnet::{self, MixnetClient};
use nym_topology::{HardcodedTopologyProvider, NymTopology};
use std::fs::File;
use std::io::Write;
use std::sync::LazyLock;
use std::time::Duration;
use std::{
    collections::VecDeque,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
};
use tokio::sync::OnceCell;
use tokio::{signal::ctrl_c, sync::RwLock};
use tokio_util::sync::CancellationToken;

static NYM_API_URL: LazyLock<String> = LazyLock::new(|| {
    std::env::var(NYM_API).unwrap_or_else(|_| panic!("{} env var not set", NYM_API))
});

static MIXNET_TIMEOUT: OnceCell<u64> = OnceCell::const_new();
static TOPOLOGY: OnceCell<NymTopology> = OnceCell::const_new();
static PRIVATE_KEY: OnceCell<PrivateKey> = OnceCell::const_new();

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

    #[arg(short, long, default_value_t = 10)]
    mixnet_timeout: u64,

    #[arg(long, default_value_t = false)]
    generate_key_pair: bool,

    #[arg(long)]
    private_key: String,
}

fn generate_key_pair() -> Result<()> {
    let mut rng = rand::thread_rng();
    let keypair = nym_crypto::asymmetric::identity::KeyPair::new(&mut rng);

    let mut public_key_file = File::create("network-monitor-public")?;
    public_key_file.write_all(keypair.public_key().to_base58_string().as_bytes())?;

    let mut private_key_file = File::create("network-monitor-private")?;
    private_key_file.write_all(keypair.private_key().to_base58_string().as_bytes())?;

    info!("Generated keypair, public key to 'network-monitor-public', and private key to 'network-monitor-private', public key should be whitelisted with the nym-api");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    nym_bin_common::logging::setup_logging();

    let args = Args::parse();

    setup_env(args.env); // Defaults to mainnet if empty

    let cancel_token = CancellationToken::new();
    let server_cancel_token = cancel_token.clone();
    let clients = Arc::new(RwLock::new(VecDeque::with_capacity(args.n_clients)));

    if args.generate_key_pair {
        generate_key_pair()?;
        std::process::exit(0);
    }

    let pk = PrivateKey::from_base58_string(&args.private_key)?;
    PRIVATE_KEY.set(pk).ok();

    TOPOLOGY
        .set(if let Some(topology_file) = args.topology {
            NymTopology::new_from_file(topology_file)?
        } else {
            NymTopology::new_from_env().await?
        })
        .ok();

    MIXNET_TIMEOUT.set(args.mixnet_timeout).ok();

    let spawn_clients = Arc::clone(&clients);
    tokio::spawn(make_clients(
        spawn_clients,
        args.n_clients,
        args.client_lifetime,
        TOPOLOGY.get().expect("Topology not set yet!").clone(),
    ));

    let server_handle = tokio::spawn(async move {
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::from_str(&args.host)?), args.port);
        let server = HttpServer::new(socket, server_cancel_token);
        server.run(clients).await
    });

    info!("Waiting for message (ctrl-c to exit)");

    loop {
        match tokio::time::timeout(Duration::from_secs(600), ctrl_c()).await {
            Ok(_) => {
                info!("Received kill signal, shutting down, submitting final batch of metrics");
                submit_metrics().await?;
                break;
            }
            Err(_) => {
                info!("Submitting metrics, cleaning metric buffers");
                submit_metrics().await?;
            }
        };
    }

    cancel_token.cancel();

    server_handle.await??;

    Ok(())
}

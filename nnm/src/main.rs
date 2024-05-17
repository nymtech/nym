use anyhow::Result;
use nym_network_defaults::setup_env;
use nym_sdk::mixnet::{self, MixnetClient};
use nym_topology::{HardcodedTopologyProvider, NymTopology};
use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::{signal::ctrl_c, sync::RwLock};

use tokio_util::sync::CancellationToken;

use crate::http::HttpServer;

mod http;

pub struct ClientWrapper {
    client: MixnetClient,
}

impl ClientWrapper {
    pub fn new(client: MixnetClient) -> Self {
        Self { client }
    }
}

async fn make_client(topology: NymTopology) -> Result<ClientWrapper> {
    let net = mixnet::NymNetworkDetails::new_from_env();
    let topology_provider = Box::new(HardcodedTopologyProvider::new(topology));

    let mixnet_client = mixnet::MixnetClientBuilder::new_ephemeral()
        .network_details(net)
        .custom_topology_provider(topology_provider)
        // .enable_credentials_mode()
        .build()?;

    let client = mixnet_client.connect_to_mixnet().await?;
    Ok(ClientWrapper::new(client))
}

#[tokio::main]
async fn main() -> Result<()> {
    nym_bin_common::logging::setup_logging();

    let args: Vec<String> = env::args().collect();
    let n_clients = if args.len() > 1 {
        args[1].parse::<usize>().unwrap()
    } else {
        32
    };

    setup_env(Some("../envs/mainnet.env"));

    let cancel_token = CancellationToken::new();

    let server_cancel_token = cancel_token.clone();

    let clients = Arc::new(RwLock::new(vec![]));

    let topology = NymTopology::new_from_file("topology.json").unwrap();

    let spawn_clients = Arc::clone(&clients);
    tokio::spawn(async move {
        for i in 0..n_clients {
            println!(
                "############################################# Making client {}",
                i
            );
            let client = match make_client(topology.clone()).await {
                Ok(client) => client,
                Err(err) => {
                    println!("{}", err);
                    continue;
                }
            };
            spawn_clients
                .write()
                .await
                .push(Arc::new(RwLock::new(client)));
        }
    });

    let _server_handle = tokio::spawn(async move {
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let server = HttpServer::new(socket, server_cancel_token);
        server.run(clients).await
    });

    ctrl_c().await?;
    println!("received ctrl-c");

    cancel_token.cancel();

    // let _ = server_handle.await;

    println!("Waiting for message (ctrl-c to exit)");

    Ok(())
}

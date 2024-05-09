use anyhow::Result;
use nym_network_defaults::setup_env;
use nym_sdk::mixnet::{self, MixnetClient};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::{signal::ctrl_c, sync::RwLock};

use tokio_util::sync::CancellationToken;

use crate::http::HttpServer;

mod http;

pub struct ClientWrapper {
    client: MixnetClient,
    rng: ChaCha8Rng,
}

impl ClientWrapper {
    pub fn new(client: MixnetClient, rng: ChaCha8Rng) -> Self {
        Self { client, rng }
    }

    pub fn rng(&mut self) -> &mut ChaCha8Rng {
        &mut self.rng
    }
}

async fn make_client() -> Result<ClientWrapper> {
    let ff_net = mixnet::NymNetworkDetails::new_from_env();

    let mixnet_client = mixnet::MixnetClientBuilder::new_ephemeral()
        .network_details(ff_net)
        // .enable_credentials_mode()
        .build()?;

    let client = mixnet_client.connect_to_mixnet().await?;
    let rng = ChaCha8Rng::seed_from_u64(1);
    Ok(ClientWrapper::new(client, rng))
}

#[tokio::main]
async fn main() -> Result<()> {
    nym_bin_common::logging::setup_logging();

    setup_env(Some("ff.env"));

    let cancel_token = CancellationToken::new();

    let server_cancel_token = cancel_token.clone();

    let mut clients = vec![];
    for i in 0..1 {
        println!(
            "############################################# Getting client {}",
            i
        );
        let client = match make_client().await {
            Ok(client) => client,
            Err(err) => {
                println!("{}", err);
                continue;
            }
        };
        clients.push(Arc::new(RwLock::new(client)));
    }

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

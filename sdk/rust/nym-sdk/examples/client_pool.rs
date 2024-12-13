// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use nym_network_defaults::setup_env;
use nym_sdk::client_pool::ClientPool;
use nym_sdk::mixnet::{MixnetClientBuilder, NymNetworkDetails};

// This client pool is used internally by the TcpProxyClient but can also be used by the Mixnet module, in case you're quickly swapping clients in and out but won't want to use the TcpProxy module for whatever reason.
//
// Run with: cargo run --example client_pool -- ../../../envs/<NETWORK>.env
#[tokio::main]
async fn main() -> Result<()> {
    nym_bin_common::logging::setup_logging();
    setup_env(std::env::args().nth(1));

    let conn_pool = ClientPool::new(1); // Start the client pool with 1 client always being kept in reserve
    let client_maker = conn_pool.clone();
    tokio::spawn(async move {
        client_maker.start().await?;
        Ok::<(), anyhow::Error>(())
    });

    let pool_clone_one = conn_pool.clone();
    let pool_clone_two = conn_pool.clone();

    tokio::spawn(async move {
        let client_one = match pool_clone_one.get_mixnet_client().await {
            Some(client) => {
                println!("Grabbed client {} from pool", client.nym_address());
                client
            }
            None => {
                println!("Not enough clients in pool, creating ephemeral client");
                let net = NymNetworkDetails::new_from_env();
                let client = MixnetClientBuilder::new_ephemeral()
                    .network_details(net)
                    .build()?
                    .connect_to_mixnet()
                    .await?;
                println!(
                    "Using {} for the moment, created outside of the connection pool",
                    client.nym_address()
                );
                client
            }
        };
        let our_address = client_one.nym_address();
        println!("Client 1: {our_address}");
        client_one.disconnect().await;
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await; // Emulate doing something
        return Ok::<(), anyhow::Error>(());
    });

    tokio::spawn(async move {
        let client_two = match pool_clone_two.get_mixnet_client().await {
            Some(client) => {
                println!("Grabbed client {} from pool", client.nym_address());
                client
            }
            None => {
                println!("Not enough clients in pool, creating ephemeral client");
                let net = NymNetworkDetails::new_from_env();
                let client = MixnetClientBuilder::new_ephemeral()
                    .network_details(net)
                    .build()?
                    .connect_to_mixnet()
                    .await?;
                println!(
                    "Using {} for the moment, created outside of the connection pool",
                    client.nym_address()
                );
                client
            }
        };
        let our_address = *client_two.nym_address();
        println!("Client 2: {our_address}");
        client_two.disconnect().await;
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await; // Emulate doing something
        return Ok::<(), anyhow::Error>(());
    });

    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    // TODO clientpool disconnect rest
    Ok(())
}

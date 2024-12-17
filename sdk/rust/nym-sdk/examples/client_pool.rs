// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use nym_network_defaults::setup_env;
use nym_sdk::client_pool::ClientPool;
use nym_sdk::mixnet::{MixnetClientBuilder, NymNetworkDetails};
use tokio::signal::ctrl_c;

// This client pool is used internally by the TcpProxyClient but can also be used by the Mixnet module, in case you're quickly swapping clients in and out but won't want to use the TcpProxy module for whatever reason.
//
// Run with: cargo run --example client_pool -- ../../../envs/<NETWORK>.env
#[tokio::main]
async fn main() -> Result<()> {
    nym_bin_common::logging::setup_logging();
    setup_env(std::env::args().nth(1));

    let conn_pool = ClientPool::new(2); // Start the Client Pool with 2 Clients always being kept in reserve
    let client_maker = conn_pool.clone();
    tokio::spawn(async move {
        client_maker.start().await?;
        Ok::<(), anyhow::Error>(())
    });

    println!("\n\nWaiting a few seconds to fill pool\n\n");
    tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;

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
                    .build()
                    .unwrap()
                    .connect_to_mixnet()
                    .await
                    .unwrap();
                println!(
                    "Using {} for the moment, created outside of the connection pool",
                    client.nym_address()
                );
                client
            }
        };
        let our_address = client_one.nym_address();
        println!("\n\nClient 1: {our_address}\n\n");
        client_one.disconnect().await;
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await; // Emulate doing something
        Ok::<(), anyhow::Error>(())
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
                    .build()
                    .unwrap()
                    .connect_to_mixnet()
                    .await
                    .unwrap();
                println!(
                    "Using {} for the moment, created outside of the connection pool",
                    client.nym_address()
                );
                client
            }
        };
        let our_address = *client_two.nym_address();
        println!("\n\nClient 2: {our_address}\n\n");
        client_two.disconnect().await;
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await; // Emulate doing something
        Ok::<(), anyhow::Error>(())
    });

    wait_for_ctrl_c(conn_pool).await?;
    Ok(())
}

async fn wait_for_ctrl_c(pool: ClientPool) -> Result<()> {
    println!("\n\nPress CTRL_C to disconnect pool\n\n");
    ctrl_c().await?;
    println!("CTRL_C received. Killing client pool");
    pool.disconnect_pool().await;
    Ok(())
}

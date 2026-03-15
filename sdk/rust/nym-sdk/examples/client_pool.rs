// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Pre-warmed pool of ephemeral Mixnet clients.
//!
//! The pool keeps a reserve of connected clients so that new connections
//! can be served instantly. When the pool is empty, a fallback ephemeral
//! client is created on-demand (with higher latency).
//!
//! Run with: cargo run --example client_pool -- ../../../envs/<NETWORK>.env

use anyhow::Result;
use nym_network_defaults::setup_env;
use nym_sdk::client_pool::ClientPool;
use nym_sdk::mixnet::{MixnetClientBuilder, NymNetworkDetails};
use tokio::signal::ctrl_c;

#[tokio::main]
async fn main() -> Result<()> {
    nym_bin_common::logging::setup_tracing_logger();
    setup_env(std::env::args().nth(1));

    // Step 1: Create a pool that maintains 2 clients in reserve.
    let conn_pool = ClientPool::new(2);

    // Step 2: Start the pool's background loop in a spawned task.
    // It will continuously create clients until the reserve is full.
    let client_maker = conn_pool.clone();
    tokio::spawn(async move {
        client_maker.start().await?;
        Ok::<(), anyhow::Error>(())
    });

    // Step 3: Wait for the pool to fill up.
    println!("\n\nWaiting a few seconds to fill pool\n\n");
    tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;

    // Step 4: Grab clients from the pool in two concurrent tasks.
    // If the pool is empty, fall back to creating an ephemeral client.
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
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
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
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        Ok::<(), anyhow::Error>(())
    });

    // Step 5: Wait for ctrl-c, then shut down the pool.
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

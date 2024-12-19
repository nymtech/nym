// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Result;
use echo_server::NymEchoServer;
use futures::stream::StreamExt;
use nym_bin_common::logging::setup_logging;
use nym_crypto::asymmetric::ed25519;
use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;
use reqwest::{self, Response, Url};
use serde_json::Value;
use std::time::Duration;
use tokio::time;
use tokio::time::timeout;

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging();
    let entry_gw_keys = reqwest_and_parse(
        Url::parse(
            "https://validator.nymtech.net/api/v1/unstable/nym-nodes/skimmed/entry-gateways/all?no_legacy=true", // make const

        )
        .unwrap(),
        "ed25519_identity_pubkey",
    )
    .await?;
    println!(
        "got {} entry gws: \n{:?}",
        entry_gw_keys.len(),
        entry_gw_keys
    );

    let exit_gw_keys = reqwest_and_parse(
        Url::parse(
            "https://validator.nymtech.net/api/v1/unstable/nym-nodes/skimmed/exit-gateways/all?no_legacy=true", // make const
        )
        .unwrap(),
        "ed25519_identity_pubkey",
    )
    .await?;
    println!(
        "got {} exit gws: \n{:?}\n",
        exit_gw_keys.len(),
        exit_gw_keys
    );

    for gw in exit_gw_keys {
        println!("{}", gw);
        time::sleep(Duration::from_secs(1)).await;

        let mut echo_server = NymEchoServer::new(
            Some(ed25519::PublicKey::from_base58_string(gw)?),
            None,
            "../../../envs/mainnet.env".to_string(), // make const
            "9000", // when you run concurrently you can probably iterate through ports here as well
        )
        .await?;

        let echo_addr = echo_server.nym_address().await;
        println!("echo addr: {echo_addr}");

        tokio::task::spawn(async move {
            echo_server.run().await?;
            Ok::<(), anyhow::Error>(())
        });

        for gw in entry_gw_keys.clone() {
            let builder = mixnet::MixnetClientBuilder::new_ephemeral()
                .request_gateway(gw)
                .build()?;

            let mut client = match builder.connect_to_mixnet().await {
                Ok(client) => {
                    println!("connected");
                    client
                }
                Err(err) => {
                    println!("failed to connect: {err}");
                    return Err(err.into());
                }
            };
            let our_address = client.nym_address();
            println!("{our_address}");
            client.send_plain_message(echo_addr, "echo").await?;

            match timeout(Duration::from_secs(5), client.next()).await {
                Err(_timeout) => {
                    println!("❌");
                    println!("timed out while waiting for the response...");
                }
                Ok(Some(received)) => match String::from_utf8(received.message) {
                    Ok(message) => {
                        println!("✅");
                        println!("received '{message}' back!");
                    }
                    Err(err) => {
                        println!("❌");
                        println!("the received message got malformed on the way to us: {err}");
                    }
                },
                Ok(None) => {
                    println!("❌");
                    println!("failed to receive any message back...");
                }
            }

            println!("disconnecting the client before shutting down...");
            client.disconnect().await;
        }

        time::sleep(Duration::from_secs(100)).await;
    }

    Ok(())
}

async fn reqwest_and_parse(endpoint: Url, key: &str) -> Result<Vec<String>> {
    let response: Response = reqwest::get(endpoint).await?;
    let json: Value = response.json().await?;
    let parsed: Vec<String> = json["nodes"]["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node[key].as_str().unwrap().to_string())
        .collect();
    Ok(parsed)
}

/*

// let response = reqwest::get(
//     "https://validator.nymtech.net/api/v1/unstable/nym-nodes/skimmed/entry-gateways/active",
// )
// .await?;
// let json: Value = response.json().await?;

// let exit_gw_keys: Vec<String> = json["nodes"]["data"]
//     .as_array()
//     .unwrap()
//     .iter()
//     .map(|node| {
//         node["ed25519_identity_pubkey"]
//             .as_str()
//             .unwrap()
//             .to_string()
//     })
//     .collect();

// println!("Got {} active exit gw keys", exit_gw_keys.len(),);

*/

// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::Result;
use echo_server::NymEchoServer;
use futures::stream::StreamExt;
use nym_bin_common::logging::setup_logging;
use nym_crypto::asymmetric::ed25519;
use nym_sdk::mixnet;
use nym_sdk::mixnet::MixnetMessageSender;
use reqwest::{self, Url};
use serde_json::Value;
use std::time::Duration;
use tokio::time;
use tokio::time::timeout;

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging();
    let entry_gw_keys = reqwest_and_parse(
        Url::parse("https://validator.nymtech.net/api/v1/unstable/nym-nodes/skimmed/entry-gateways/all?no_legacy=true").unwrap(),
        "EntryGateway",
    )
    .await?;
    println!(
        "got {} entry gws: \n{:?}",
        entry_gw_keys.len(),
        entry_gw_keys
    );

    let exit_gw_keys = reqwest_and_parse(
        Url::parse(
            "https://validator.nymtech.net/api/v1/unstable/nym-nodes/skimmed/exit-gateways/all?no_legacy=true",
        )
        .unwrap(),
        "ExitGateway",
    )
    .await?;
    println!(
        "got {} exit gws: \n{:?}\n",
        exit_gw_keys.len(),
        exit_gw_keys
    );

    for gw in exit_gw_keys {
        println!("{}", gw);

        // TODO set up a client manually with a reply fn to troubleshoot wtf is going on

        // time::sleep(Duration::from_secs(1)).await;
        // let mut echo_server = NymEchoServer::new(
        //     Some(ed25519::PublicKey::from_base58_string(
        //         exit_gw_keys[0].clone(),
        //     )?),
        //     None,
        //     "../../../envs/mainnet.env".to_string(), // make const
        //     "9000", // when you run concurrently you can probably iterate through ports here as well
        // )
        // .await?;

        // let echo_addr = echo_server.nym_address().await;
        // println!("echo addr: {echo_addr}");

        // tokio::task::spawn(async move {
        //     echo_server.run().await?;
        //     Ok::<(), anyhow::Error>(())
        // });

        for gw in entry_gw_keys.clone() {
            let builder = mixnet::MixnetClientBuilder::new_ephemeral()
                .request_gateway(gw)
                .build()?;

            let mut client = match builder.connect_to_mixnet().await {
                Ok(client) => client,
                Err(err) => {
                    println!("failed to connect: {err}");
                    return Err(err.into());
                }
            };
            let our_address = client.nym_address();
            println!("{our_address}");
            client.send_plain_message(*our_address, "echo").await?;

            match timeout(Duration::from_secs(5), client.next()).await {
                Err(_timeout) => {
                    println!("timed out");
                }
                Ok(Some(received)) => match String::from_utf8(received.message) {
                    Ok(message) => {
                        println!("received '{message}' back!");
                    }
                    Err(err) => {
                        println!("the received message got malformed on the way to us: {err}");
                    }
                },
                Ok(None) => {
                    println!("failed to receive any message back...");
                }
            }
            println!("disconnecting the client before shutting down...");
            client.disconnect().await;
        }
    }

    Ok(())
}

async fn reqwest_and_parse(endpoint: Url, key: &str) -> Result<Vec<String>> {
    let response = reqwest::get(endpoint).await?;
    let json: Value = response.json().await?;
    let filtered_keys = filter_gateway_keys(&json, key)?;
    Ok(filtered_keys)
}

fn filter_gateway_keys(json: &Value, key: &str) -> Result<Vec<String>> {
    let mut filtered_keys = Vec::new();

    if let Some(nodes) = json["nodes"]["data"].as_array() {
        for node in nodes {
            if let Some(performance) = node.get("performance").and_then(|v| v.as_str()) {
                let performance_value: f64 = performance.parse().unwrap_or(0.0);

                let inactive = node.get("role").and_then(|v| v.as_str()) == Some("Inactive");

                if let Some(role) = node.get("role").and_then(|v| v.as_str()) {
                    let is_correct_gateway = role == key;
                    // println!("Node: {:?}", node);
                    // println!("Performance: {}", performance_value);
                    // println!("Blacklisted: {}", inactive);
                    if performance_value > 0.0 && !inactive && is_correct_gateway {
                        if let Some(gateway_identity_key) =
                            node.get("ed25519_identity_pubkey").and_then(|v| v.as_str())
                        {
                            filtered_keys.push(gateway_identity_key.to_string());
                        }
                    }
                }
            }
        }
    } else {
        println!("No nodes found ");
    }
    Ok(filtered_keys)
}

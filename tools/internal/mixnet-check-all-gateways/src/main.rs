// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::{anyhow, Result};
use echo_server::NymEchoServer;
use futures::stream::StreamExt;
use nym_bin_common::logging::setup_logging;
use nym_crypto::asymmetric::ed25519;
use nym_sdk::mixnet;
use nym_sdk::mixnet::{
    AnonymousSenderTag, IncludedSurbs, MixnetMessageSender, ReconstructedMessage,
};
use reqwest::{self, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::Write;
use std::time::Duration;
use tokio::time::timeout;
use tokio::time::{self, Timeout};
#[path = "utils.rs"]
// TODO make these exportable from tcp_proxy module and then import from there, ditto with echo server lib
mod utils;
use utils::{Payload, ProxiedMessage};

const TIMEOUT: u64 = 10; // message ping timeout

#[derive(Serialize, Deserialize, Debug)]
struct TestResult {
    entry_gw: String,
    exit_gw: String,
    error: TestError,
}

#[derive(Serialize, Deserialize, Debug)]
enum TestError {
    Timeout,
    NoMessage,
    None,
    CouldNotCreateEchoServer(String),
    Other(String),
}

#[tokio::main]
async fn main() -> Result<()> {
    // setup_logging(); // TODO think about parsing and noise here, could just parse on errors from all libs and then have info from here?
    let entry_gw_keys = reqwest_and_parse(
        Url::parse("https://validator.nymtech.net/api/v1/unstable/nym-nodes/skimmed/entry-gateways/all?no_legacy=true").unwrap(),
        "EntryGateway",
    )
    .await?;
    // println!(
    //     "got {} entry gws: \n{:?}",
    //     entry_gw_keys.len(),
    //     entry_gw_keys
    // );
    println!("got {} entry gws", entry_gw_keys.len(),);

    let exit_gw_keys = reqwest_and_parse(
        Url::parse(
            "https://validator.nymtech.net/api/v1/unstable/nym-nodes/skimmed/exit-gateways/all?no_legacy=true",
        )
        .unwrap(),
        "ExitGateway",
    )
    .await?;
    // println!(
    //     "got {} exit gws: \n{:?}\n",
    //     exit_gw_keys.len(),
    //     exit_gw_keys
    // );
    println!("got {} exit gws", exit_gw_keys.len(),);

    let mut port_range: u64 = 9000; // port that we start iterating upwards from, will go from port_range to (port_range + exit_gws.len()) by the end of the run

    for exit_gw in exit_gw_keys.clone() {
        println!("creating echo server connecting to {}", exit_gw);

        let filepath = format!("./src/results/{}.json", exit_gw.clone());
        let mut results = OpenOptions::new()
            .read(true)
            .write(true) // .append(true)
            .create(true)
            .open(filepath)?;

        let mut results_vec: Vec<Value> = Vec::new();
        let home_dir = dirs::home_dir().expect("Unable to get home directory");
        let mut echo_server = match NymEchoServer::new(
            Some(ed25519::PublicKey::from_base58_string(&exit_gw)?),
            Some(
                format!(
                    "{}/tmp/nym-proxy-server-config-{}",
                    home_dir.display(),
                    &exit_gw
                )
                .as_str(),
            ),
            "../../../envs/mainnet.env".to_string(), // make const
            port_range.to_string().as_str(),
        )
        .await
        {
            Ok(echo_server) => echo_server,
            Err(err) => {
                let res = TestResult {
                    entry_gw: "".to_string(),
                    exit_gw: exit_gw.clone(),
                    error: TestError::CouldNotCreateEchoServer(err.to_string()),
                };
                results_vec.push(json!(res));
                let json_array = json!(results_vec);
                println!("{json_array}");
                results.write_all(json_array.to_string().as_bytes())?;
                continue;
            }
        };
        port_range += 1;

        let echo_addr = echo_server.nym_address().await;
        println!("echo addr: {echo_addr}");

        tokio::task::spawn(async move {
            echo_server.run().await?;
            Ok::<(), anyhow::Error>(())
        });

        // dumb sleep to let it startup
        time::sleep(Duration::from_secs(5)).await;

        for entry_gw in entry_gw_keys.clone() {
            let builder = mixnet::MixnetClientBuilder::new_ephemeral()
                .request_gateway(entry_gw.clone())
                .build()?;

            let mut client = match builder.connect_to_mixnet().await {
                Ok(client) => client,
                Err(err) => {
                    let res = TestResult {
                        entry_gw: entry_gw.clone(),
                        exit_gw: exit_gw.clone(),
                        error: TestError::Other(err.to_string()),
                    };
                    println!("{res:#?}");
                    results_vec.push(json!(res));
                    // println!("failed to connect: {err}");
                    continue;
                }
            };

            let test_address = client.nym_address();
            println!("currently testing entry gateway: {test_address}");

            // Has to be ProxiedMessage for the moment which is slightly annoying until I
            // modify the ProxyServer to just stupidly echo back whatever it gets in a
            // ReconstructedMessage format if it can't deseralise it to a ProxiedMessage
            let session_id = uuid::Uuid::new_v4();
            let message_id = 0;
            let outgoing = ProxiedMessage::new(
                Payload::Data("echo test".as_bytes().to_vec()),
                session_id,
                message_id,
            );
            let coded_message = bincode::serialize(&outgoing).unwrap();

            match client
                .send_message(echo_addr, &coded_message, IncludedSurbs::Amount(10))
                .await
            {
                Ok(_) => {
                    println!("Message sent");
                }
                Err(err) => {
                    let res = TestResult {
                        entry_gw: entry_gw.clone(),
                        exit_gw: exit_gw.clone(),
                        error: TestError::Other(err.to_string()),
                    };
                    println!("{res:#?}");
                    results_vec.push(json!(res));
                    continue;
                }
            };

            let res = match timeout(Duration::from_secs(TIMEOUT), client.next()).await {
                Err(_timeout) => {
                    println!("timed out");
                    TestResult {
                        entry_gw,
                        exit_gw: exit_gw.clone(),
                        error: TestError::Timeout,
                    }
                }
                Ok(Some(received)) => {
                    let incoming: ProxiedMessage = bincode::deserialize(&received.message).unwrap();
                    println!("\ngot echo: {incoming}\n");
                    // TODO check incoming is same as outgoing else make a MangledReply err type
                    TestResult {
                        entry_gw,
                        exit_gw: exit_gw.clone(),
                        error: TestError::None,
                    }
                }
                Ok(None) => {
                    println!("failed to receive any message back...");
                    TestResult {
                        entry_gw,
                        exit_gw: exit_gw.clone(),
                        error: TestError::NoMessage,
                    }
                }
            };
            println!("{res:#?}");
            results_vec.push(json!(res));
            println!("disconnecting the client before shutting down...");
            client.disconnect().await;
        }
        let json_array = json!(results_vec);
        println!("{json_array}");
        results.write_all(json_array.to_string().as_bytes())?;
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
                let performance_value: f64 = performance.parse().unwrap_or(0.0); // TODO could make this a const @ top?

                let inactive = node.get("role").and_then(|v| v.as_str()) == Some("Inactive");

                if let Some(role) = node.get("role").and_then(|v| v.as_str()) {
                    let is_correct_gateway = role == key;
                    // println!("node addr: {:?}", node);
                    // println!("perf score: {}", performance_value);
                    // println!("status: {}", inactive);
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
        // TODO make error and return / break
        // println!("No nodes found ");
        return Err(anyhow!("Could not parse any gateways"));
    }
    Ok(filtered_keys)
}

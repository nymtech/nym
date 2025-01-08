// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use anyhow::{anyhow, Result};
use echo_server::NymEchoServer;
use futures::stream::StreamExt;
use nym_bin_common::logging::setup_logging;
use nym_crypto::asymmetric::ed25519;
use nym_sdk::mixnet;
use nym_sdk::mixnet::{IncludedSurbs, MixnetMessageSender};
use reqwest::{self, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::Duration;
use tokio::signal;
use tokio::time;
use tokio::time::timeout;
#[path = "utils.rs"]
// TODO make these exportable from tcp_proxy module and then import from there, ditto with echo server lib
mod utils;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};
use utils::{Payload, ProxiedMessage};

const TIMEOUT: u64 = 10; // message ping timeout
const MESSAGE: &str = "echo test";

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
    setup_logging(); // TODO think about parsing and noise here if make it concurrent. Could just parse on errors from all libs and then have info from here? echo server metrics + info logging from this code + error logs from elsewhere should be ok
    let entry_gw_keys = reqwest_and_parse(
        Url::parse("https://validator.nymtech.net/api/v1/unstable/nym-nodes/skimmed/entry-gateways/all?no_legacy=true").unwrap(),
        "EntryGateway",
    )
    .await?;
    debug!(
        "got {} entry gws: \n{:?}",
        entry_gw_keys.len(),
        entry_gw_keys
    );
    info!("got {} entry gws", entry_gw_keys.len(),);

    let exit_gw_keys = reqwest_and_parse(
        Url::parse(
            "https://validator.nymtech.net/api/v1/unstable/nym-nodes/skimmed/exit-gateways/all?no_legacy=true",
        )
        .unwrap(),
        "ExitGateway",
    )
    .await?;
    debug!(
        "got {} exit gws: \n{:?}\n",
        exit_gw_keys.len(),
        exit_gw_keys
    );
    info!("got {} exit gws", exit_gw_keys.len(),);

    let mut port_range: u64 = 9000; // Port that we start iterating upwards from, will go from port_range to (port_range + exit_gws.len()) by the end of the run. This was made configurable presuming at some point we'd try make this run concurrently for speedup.

    let cancel_token = CancellationToken::new();
    let watcher_token = cancel_token.clone();
    tokio::spawn(async move {
        signal::ctrl_c().await?;
        println!("CTRL_C received");
        watcher_token.cancel();
        Ok::<(), anyhow::Error>(())
    });

    let start = SystemTime::now();
    let time_now = start.duration_since(UNIX_EPOCH).unwrap().as_secs();

    for exit_gw in exit_gw_keys.clone() {
        let loop_token = cancel_token.child_token();
        let inner_loop_token = cancel_token.child_token();
        if loop_token.is_cancelled() {
            break;
        }
        let thread_token = cancel_token.child_token();
        let last_check_token = thread_token.clone();

        if !fs::metadata(format!("./src/results/{}", time_now))
            .map(|metadata| metadata.is_dir())
            .unwrap_or(false)
        {
            fs::create_dir_all(format!("./src/results/{}", time_now))?;
        }

        info!("creating echo server connecting to {}", exit_gw);

        let filepath = format!("./src/results/{}/{}.json", time_now, exit_gw.clone());
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
            "../../../envs/mainnet.env".to_string(), // TODO replace with None
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
                info!("{json_array}");
                results.write_all(json_array.to_string().as_bytes())?;
                continue;
            }
        };
        port_range += 1;

        let echo_disconnect_signal = echo_server.disconnect_signal();
        let echo_addr = echo_server.nym_address().await;
        debug!("echo addr: {echo_addr}");

        tokio::task::spawn(async move {
            loop {
                tokio::select! {
                    _ = thread_token.cancelled() => {
                        info!("loop over; disconnecting echo server {}", echo_addr.clone());
                        echo_disconnect_signal.send(()).await?;
                        break;
                    }
                    _ = echo_server.run() => {}
                }
            }
            Ok::<(), anyhow::Error>(())
        });

        // dumb sleep to let it startup
        time::sleep(Duration::from_secs(5)).await;

        for entry_gw in entry_gw_keys.clone() {
            if inner_loop_token.is_cancelled() {
                info!("Inner loop cancelled");
                break;
            }
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
                    info!("{res:#?}");
                    results_vec.push(json!(res));
                    continue;
                }
            };

            let test_address = client.nym_address();
            info!("currently testing entry gateway: {test_address}");

            // Has to be ProxiedMessage for the moment which is slightly annoying until I
            // modify the ProxyServer to just stupidly echo back whatever it gets in a
            // ReconstructedMessage format if it can't deseralise incoming traffic to a ProxiedMessage
            let session_id = uuid::Uuid::new_v4();
            let message_id = 0;
            let outgoing = ProxiedMessage::new(
                Payload::Data(MESSAGE.as_bytes().to_vec()),
                session_id,
                message_id,
            );
            let coded_message = bincode::serialize(&outgoing).unwrap();

            match client
                .send_message(echo_addr, &coded_message, IncludedSurbs::Amount(30))
                .await
            {
                Ok(_) => {
                    debug!("Message sent");
                }
                Err(err) => {
                    let res = TestResult {
                        entry_gw: entry_gw.clone(),
                        exit_gw: exit_gw.clone(),
                        error: TestError::Other(err.to_string()),
                    };
                    info!("{res:#?}");
                    results_vec.push(json!(res));
                    continue;
                }
            };

            let res = match timeout(Duration::from_secs(TIMEOUT), client.next()).await {
                Err(_timeout) => {
                    warn!("timed out");
                    TestResult {
                        entry_gw: entry_gw.clone(),
                        exit_gw: exit_gw.clone(),
                        error: TestError::Timeout,
                    }
                }
                Ok(Some(received)) => {
                    let incoming: ProxiedMessage = bincode::deserialize(&received.message).unwrap();
                    info!("got echo: {:?}", incoming);
                    debug!(
                        "sent message as lazy ref until I properly sort the utils for comparison: {:?}",
                        MESSAGE.as_bytes()
                    );
                    debug!("incoming message: {:?}", incoming.message);
                    TestResult {
                        entry_gw: entry_gw.clone(),
                        exit_gw: exit_gw.clone(),
                        error: TestError::None,
                    }
                }
                Ok(None) => {
                    info!("failed to receive any message back...");
                    TestResult {
                        entry_gw: entry_gw.clone(),
                        exit_gw: exit_gw.clone(),
                        error: TestError::NoMessage,
                    }
                }
            };
            debug!("{res:#?}");
            results_vec.push(json!(res));
            debug!("disconnecting the client before shutting down...");
            client.disconnect().await;

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            info!("{}", &entry_gw);

            if Some(&entry_gw) == entry_gw_keys.last() {
                last_check_token.cancel();
            }
        }
        let json_array = json!(results_vec);
        debug!("{json_array}");
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
                let performance_value: f64 = performance.parse().unwrap_or(0.0); // TODO make this configurable?

                let inactive = node.get("role").and_then(|v| v.as_str()) == Some("Inactive");

                if let Some(role) = node.get("role").and_then(|v| v.as_str()) {
                    let is_correct_gateway = role == key;
                    debug!("node addr: {:?}", node);
                    debug!("perf score: {}", performance_value);
                    debug!("status: {}", inactive);
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
        return Err(anyhow!("Could not parse any gateways"));
    }
    Ok(filtered_keys)
}

// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::measurements::metrics::SharedVerlocStats;
use crate::measurements::sender::TestedNode;
use crate::measurements::{Config, PacketListener, PacketSender};
use crate::models::VerlocNodeResult;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use nym_crypto::asymmetric::ed25519;
use nym_task::ShutdownToken;
use nym_validator_client::models::NymNodeDescription;
use nym_validator_client::nym_api::NymApiClientExt;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::{debug, error, info, trace, warn};

pub struct VerlocMeasurer {
    config: Config,
    packet_sender: Arc<PacketSender>,
    packet_listener: Arc<PacketListener>,
    shutdown_token: ShutdownToken,
    state: SharedVerlocStats,
}

impl VerlocMeasurer {
    pub fn new(
        config: Config,
        identity: Arc<ed25519::KeyPair>,
        shutdown_token: ShutdownToken,
    ) -> Self {
        VerlocMeasurer {
            packet_sender: Arc::new(PacketSender::new(
                Arc::clone(&identity),
                config.packets_per_node,
                config.packet_timeout,
                config.connection_timeout,
                config.delay_between_packets,
                shutdown_token.clone_with_suffix("packet_sender"),
            )),
            packet_listener: Arc::new(PacketListener::new(
                config.listening_address,
                Arc::clone(&identity),
                shutdown_token.clone_with_suffix("packet_listener"),
            )),
            shutdown_token,
            config,
            state: SharedVerlocStats::default(),
        }
    }

    pub fn set_shared_state(&mut self, state: SharedVerlocStats) {
        self.state = state;
    }

    fn start_listening(&self) -> JoinHandle<()> {
        let packet_listener = Arc::clone(&self.packet_listener);
        tokio::spawn(packet_listener.run())
    }

    async fn perform_measurement(&self, nodes_to_test: Vec<TestedNode>) -> MeasurementOutcome {
        trace!("Performing measurements");
        if nodes_to_test.is_empty() {
            debug!("there are no nodes to measure");
            return MeasurementOutcome::Done;
        }

        for chunk in nodes_to_test.chunks(self.config.tested_nodes_batch_size) {
            let mut chunk_results = Vec::with_capacity(chunk.len());

            let mut measurement_chunk = chunk
                .iter()
                .map(|node| {
                    let node = *node;
                    let packet_sender = Arc::clone(&self.packet_sender);
                    // TODO: there's a potential issue here. if we make the measurement go into separate
                    // task, we risk biasing results with the bunch of context switches overhead
                    // but if we don't do it, it will take ages to complete

                    // TODO: check performance difference when it's not spawned as a separate task
                    tokio::spawn(async move {
                        (
                            packet_sender.send_packets_to_node(node).await,
                            node.identity,
                        )
                    })
                })
                .collect::<FuturesUnordered<_>>();

            // exhaust the results
            while !self.shutdown_token.is_cancelled() {
                tokio::select! {
                    measurement_result = measurement_chunk.next() => {
                        let Some(result) = measurement_result else {
                            // if the stream has finished, it means we got everything we could have gotten
                            break
                        };

                        // if we receive JoinError it means the task failed to get executed, so either there's a bigger issue with tokio
                        // or there was a panic inside the task itself. In either case, we should just terminate ourselves.
                        let Ok(execution_result) = result else {
                            error!("the verloc measurer has panicked!");
                            continue
                        };
                        let identity = execution_result.1;

                        let measurement_result = match execution_result.0 {
                            Err(err) => {
                                debug!("Failed to perform measurement for {identity}: {err}");
                                None
                            }
                            Ok(result) => Some(result),
                        };
                        chunk_results.push(VerlocNodeResult::new(identity, measurement_result));
                    },
                    _ = self.shutdown_token.cancelled() => {
                        trace!("Shutdown received while measuring");
                        return MeasurementOutcome::Shutdown;
                    }
                }
            }

            // update the results vector with chunks as they become available (by default every 50 nodes)
            self.state.append_measurement_results(chunk_results).await;
        }

        MeasurementOutcome::Done
    }

    async fn get_list_of_nodes(&self) -> Option<Vec<NymNodeDescription>> {
        let mut api_endpoints = self.config.nym_api_urls.clone();
        api_endpoints.shuffle(&mut thread_rng());
        for api_endpoint in api_endpoints {
            let client =
                match nym_http_api_client::Client::builder(api_endpoint.clone()).and_then(|b| {
                    b.with_user_agent(self.config.user_agent.clone())
                        .build::<nym_api_requests::models::RequestError>()
                }) {
                    Ok(c) => c,
                    Err(err) => {
                        warn!("failed to create client for {api_endpoint}: {err}");
                        continue;
                    }
                };
            match client.get_all_described_nodes().await {
                Ok(res) => return Some(res),
                Err(err) => {
                    warn!("failed to get described nodes from {api_endpoint}: {err}")
                }
            }
        }
        None
    }

    pub async fn run(&mut self) {
        self.start_listening();

        while !self.shutdown_token.is_cancelled() {
            info!("Starting verloc measurements");
            // TODO: should we also measure gateways?

            let Some(all_nodes) = self.get_list_of_nodes().await else {
                error!("failed to obtain list of all nodes from any available api endpoint");
                sleep(self.config.retry_timeout).await;
                continue;
            };

            if all_nodes.is_empty() {
                warn!("it does not seem there are any nodes to measure...");
                sleep(self.config.retry_timeout).await;
                continue;
            }

            // we only care about address and identity
            let tested_nodes = all_nodes
                .into_iter()
                .filter_map(|node| {
                    // try to parse the identity and host
                    let node_identity = node.ed25519_identity_key();

                    let ip = node.description.host_information.ip_address.first()?;
                    let verloc_port = node.description.verloc_port();
                    let verloc_host = SocketAddr::new(*ip, verloc_port);

                    // TODO: possible problem in the future, this does name resolution and theoretically
                    // if a lot of nodes maliciously mis-configured themselves, it might take a while to resolve them all
                    // However, maybe it's not a problem as if they are misconfigured, they will eventually be
                    // pushed out of the network and on top of that, verloc is done in separate task that runs
                    // only every few hours.
                    Some(TestedNode::new(verloc_host, node_identity))
                })
                .collect::<Vec<_>>();

            // on start of each run remove old results
            self.state.start_new_measurements(tested_nodes.len()).await;

            if let MeasurementOutcome::Shutdown = self.perform_measurement(tested_nodes).await {
                trace!("Shutting down after aborting measurements");
                break;
            }

            // write current time to "run finished" field
            self.state.finish_measurements().await;

            info!(
                "Finished performing verloc measurements. The next one will happen in {:?}",
                self.config.testing_interval
            );

            tokio::select! {
                _ = sleep(self.config.testing_interval) => {},
                _ = self.shutdown_token.cancelled() => {
                    trace!("Shutdown received while sleeping");
                }
            }
        }

        trace!("Verloc: Exiting");
    }
}

enum MeasurementOutcome {
    Done,
    Shutdown,
}

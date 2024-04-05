// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::verloc::listener::PacketListener;
use crate::verloc::sender::{PacketSender, TestedNode};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::*;
use nym_bin_common::version_checker::{self, parse_version};
use nym_crypto::asymmetric::identity;
use nym_network_defaults::mainnet::NYM_API;
use nym_node_http_api::state::metrics::{SharedVerlocStats, VerlocNodeResult};
use nym_task::TaskClient;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use url::Url;

use measurement::VerlocStatsUpdateExt;

// pub use crate::verloc::measurement::{AtomicVerlocResult, Verloc, VerlocResult};

pub mod error;
pub(crate) mod listener;
pub(crate) mod measurement;
pub(crate) mod packet;
pub(crate) mod sender;

// TODO: MUST BE UPDATED BEFORE ACTUAL RELEASE!!
pub const MINIMUM_NODE_VERSION: &str = "0.10.1";

// by default all of those are overwritten by config data from mixnodes directly
const DEFAULT_VERLOC_PORT: u16 = 1790;
const DEFAULT_PACKETS_PER_NODE: usize = 100;
const DEFAULT_PACKET_TIMEOUT: Duration = Duration::from_millis(1500);
const DEFAULT_CONNECTION_TIMEOUT: Duration = Duration::from_millis(5000);
const DEFAULT_DELAY_BETWEEN_PACKETS: Duration = Duration::from_millis(50);
const DEFAULT_BATCH_SIZE: usize = 50;
const DEFAULT_TESTING_INTERVAL: Duration = Duration::from_secs(60 * 60 * 12);
const DEFAULT_RETRY_TIMEOUT: Duration = Duration::from_secs(60 * 30);

#[derive(Clone, Debug)]
pub struct Config {
    /// Minimum semver version of a node (gateway or mixnode) that is capable of replying to echo packets.
    minimum_compatible_node_version: version_checker::Version,

    /// Socket address of this node on which it will be listening for the measurement packets.
    listening_address: SocketAddr,

    /// Specifies number of echo packets sent to each node during a measurement run.
    packets_per_node: usize,

    /// Specifies maximum amount of time to wait for the reply packet to arrive before abandoning the test.
    packet_timeout: Duration,

    /// Specifies maximum amount of time to wait for the connection to get established.
    connection_timeout: Duration,

    /// Specifies delay between subsequent test packets being sent (after receiving a reply).
    delay_between_packets: Duration,

    /// Specifies number of nodes being tested at once.
    tested_nodes_batch_size: usize,

    /// Specifies delay between subsequent test runs.
    testing_interval: Duration,

    /// Specifies delay between attempting to run the measurement again if the previous run failed
    /// due to being unable to get the list of nodes.
    retry_timeout: Duration,

    /// URLs to the nym apis for obtaining network topology.
    nym_api_urls: Vec<Url>,
}

impl Config {
    pub fn build() -> ConfigBuilder {
        ConfigBuilder::new()
    }
}

#[must_use]
pub struct ConfigBuilder(Config);

impl ConfigBuilder {
    pub fn new() -> ConfigBuilder {
        Self::default()
    }

    pub fn minimum_compatible_node_version(mut self, version: version_checker::Version) -> Self {
        self.0.minimum_compatible_node_version = version;
        self
    }

    pub fn listening_address(mut self, listening_address: SocketAddr) -> Self {
        self.0.listening_address = listening_address;
        self
    }

    pub fn packets_per_node(mut self, packets_per_node: usize) -> Self {
        self.0.packets_per_node = packets_per_node;
        self
    }

    pub fn packet_timeout(mut self, packet_timeout: Duration) -> Self {
        self.0.packet_timeout = packet_timeout;
        self
    }

    pub fn connection_timeout(mut self, connection_timeout: Duration) -> Self {
        self.0.connection_timeout = connection_timeout;
        self
    }

    pub fn delay_between_packets(mut self, delay_between_packets: Duration) -> Self {
        self.0.delay_between_packets = delay_between_packets;
        self
    }

    pub fn tested_nodes_batch_size(mut self, tested_nodes_batch_size: usize) -> Self {
        self.0.tested_nodes_batch_size = tested_nodes_batch_size;
        self
    }

    pub fn testing_interval(mut self, testing_interval: Duration) -> Self {
        self.0.testing_interval = testing_interval;
        self
    }

    pub fn retry_timeout(mut self, retry_timeout: Duration) -> Self {
        self.0.retry_timeout = retry_timeout;
        self
    }

    pub fn nym_api_urls(mut self, nym_api_urls: Vec<Url>) -> Self {
        self.0.nym_api_urls = nym_api_urls;
        self
    }

    pub fn build(self) -> Config {
        // panics here are fine as those are only ever constructed at the initial setup
        assert!(
            !self.0.nym_api_urls.is_empty(),
            "at least one validator endpoint must be provided",
        );
        self.0
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        ConfigBuilder(Config {
            minimum_compatible_node_version: parse_version(MINIMUM_NODE_VERSION).unwrap(),
            listening_address: format!("[::]:{DEFAULT_VERLOC_PORT}").parse().unwrap(),
            packets_per_node: DEFAULT_PACKETS_PER_NODE,
            packet_timeout: DEFAULT_PACKET_TIMEOUT,
            connection_timeout: DEFAULT_CONNECTION_TIMEOUT,
            delay_between_packets: DEFAULT_DELAY_BETWEEN_PACKETS,
            tested_nodes_batch_size: DEFAULT_BATCH_SIZE,
            testing_interval: DEFAULT_TESTING_INTERVAL,
            retry_timeout: DEFAULT_RETRY_TIMEOUT,
            nym_api_urls: vec![NYM_API.parse().expect("Invalid default API URL")],
        })
    }
}

pub struct VerlocMeasurer {
    config: Config,
    packet_sender: Arc<PacketSender>,
    packet_listener: Arc<PacketListener>,
    shutdown_listener: TaskClient,

    currently_used_api: usize,

    // Note: this client is only fine here as it does not maintain constant connection to the validator.
    // It only does bunch of REST queries. If we update it at some point to a more sophisticated (maybe signing) client,
    // then it definitely cannot be constructed here and probably will need to be passed from outside,
    // as mixnodes/gateways would already be using an instance of said client.
    validator_client: nym_validator_client::NymApiClient,
    state: SharedVerlocStats,
}

impl VerlocMeasurer {
    pub fn new(
        mut config: Config,
        identity: Arc<identity::KeyPair>,
        shutdown_listener: TaskClient,
    ) -> Self {
        config.nym_api_urls.shuffle(&mut thread_rng());

        VerlocMeasurer {
            packet_sender: Arc::new(PacketSender::new(
                Arc::clone(&identity),
                config.packets_per_node,
                config.packet_timeout,
                config.connection_timeout,
                config.delay_between_packets,
                shutdown_listener.clone().named("VerlocPacketSender"),
            )),
            packet_listener: Arc::new(PacketListener::new(
                config.listening_address,
                Arc::clone(&identity),
                shutdown_listener.clone().named("VerlocPacketListener"),
            )),
            shutdown_listener,
            currently_used_api: 0,
            validator_client: nym_validator_client::NymApiClient::new(
                config.nym_api_urls[0].clone(),
            ),
            config,
            state: SharedVerlocStats::default(),
        }
    }

    pub fn set_shared_state(&mut self, state: SharedVerlocStats) {
        self.state = state;
    }

    fn use_next_nym_api(&mut self) {
        if self.config.nym_api_urls.len() == 1 {
            warn!("There's only a single validator API available - it won't be possible to use a different one");
            return;
        }

        self.currently_used_api = (self.currently_used_api + 1) % self.config.nym_api_urls.len();
        self.validator_client
            .change_nym_api(self.config.nym_api_urls[self.currently_used_api].clone())
    }

    fn start_listening(&self) -> JoinHandle<()> {
        let packet_listener = Arc::clone(&self.packet_listener);
        tokio::spawn(packet_listener.run())
    }

    async fn perform_measurement(&self, nodes_to_test: Vec<TestedNode>) -> MeasurementOutcome {
        log::trace!("Performing measurements");
        if nodes_to_test.is_empty() {
            log::debug!("there are no nodes to measure");
            return MeasurementOutcome::Done;
        }

        let mut shutdown_listener = self.shutdown_listener.clone().named("VerlocMeasurement");
        shutdown_listener.mark_as_success();

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
            while !shutdown_listener.is_shutdown() {
                tokio::select! {
                    measurement_result = measurement_chunk.next() => {
                        let Some(result) = measurement_result else {
                            // if the stream has finished, it means we got everything we could have gotten
                            break
                        };

                        // if we receive JoinError it means the task failed to get executed, so either there's a bigger issue with tokio
                        // or there was a panic inside the task itself. In either case, we should just terminate ourselves.
                        let execution_result = result.expect("the measurement task panicked!");
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
                    _ = shutdown_listener.recv() => {
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

    pub async fn run(&mut self) {
        self.start_listening();

        while !self.shutdown_listener.is_shutdown() {
            info!("Starting verloc measurements");
            // TODO: should we also measure gateways?

            let all_mixes = match self.validator_client.get_cached_mixnodes().await {
                Ok(nodes) => nodes,
                Err(err) => {
                    error!(
                        "failed to obtain list of mixnodes from the validator - {}. Going to attempt to use another validator API in the next run",
                        err
                    );
                    self.use_next_nym_api();
                    sleep(self.config.retry_timeout).await;
                    continue;
                }
            };
            if all_mixes.is_empty() {
                warn!("There does not seem there are any nodes to measure...")
            }

            // we only care about address and identity
            let tested_nodes = all_mixes
                .into_iter()
                .filter_map(|node| {
                    let mix_node = node.bond_information.mix_node;
                    // check if the node has sufficient version to be able to understand the packets
                    let node_version = parse_version(&mix_node.version).ok()?;
                    if node_version < self.config.minimum_compatible_node_version {
                        return None;
                    }

                    // try to parse the identity and host
                    let node_identity =
                        identity::PublicKey::from_base58_string(mix_node.identity_key).ok()?;

                    let verloc_host = (&*mix_node.host, mix_node.verloc_port)
                        .to_socket_addrs()
                        .ok()?
                        .next()?;

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
                log::trace!("Shutting down after aborting measurements");
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
                _ = self.shutdown_listener.recv() => {
                    log::trace!("Shutdown received while sleeping");
                }
            }
        }

        log::trace!("Verloc: Exiting");
    }
}

enum MeasurementOutcome {
    Done,
    Shutdown,
}

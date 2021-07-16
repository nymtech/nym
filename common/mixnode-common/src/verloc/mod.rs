// Copyright 2021 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::verloc::listener::PacketListener;
pub use crate::verloc::measurement::{AtomicVerlocResult, Verloc, VerlocResult};
use crate::verloc::sender::{PacketSender, TestedNode};
use crypto::asymmetric::identity;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::*;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use version_checker::parse_version;

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

    /// URLs to the validator servers for obtaining network topology.
    validator_urls: Vec<String>,

    /// Address of the validator contract managing the network.
    mixnet_contract_address: String,
}

impl Config {
    pub fn build() -> ConfigBuilder {
        ConfigBuilder::new()
    }
}

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
    pub fn validator_urls(mut self, validator_urls: Vec<String>) -> Self {
        self.0.validator_urls = validator_urls;
        self
    }
    pub fn mixnet_contract_address<S: Into<String>>(mut self, mixnet_contract_address: S) -> Self {
        self.0.mixnet_contract_address = mixnet_contract_address.into();
        self
    }
    pub fn build(self) -> Config {
        // panics here are fine as those are only ever constructed at the initial setup
        if self.0.validator_urls.is_empty() {
            panic!("at least one validator endpoint must be provided")
        }
        if self.0.mixnet_contract_address.is_empty() {
            panic!("the mixnet contract address must be set")
        }
        self.0
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        ConfigBuilder(Config {
            minimum_compatible_node_version: parse_version(MINIMUM_NODE_VERSION).unwrap(),
            listening_address: format!("[::]:{}", DEFAULT_VERLOC_PORT).parse().unwrap(),
            packets_per_node: DEFAULT_PACKETS_PER_NODE,
            packet_timeout: DEFAULT_PACKET_TIMEOUT,
            connection_timeout: DEFAULT_CONNECTION_TIMEOUT,
            delay_between_packets: DEFAULT_DELAY_BETWEEN_PACKETS,
            tested_nodes_batch_size: DEFAULT_BATCH_SIZE,
            testing_interval: DEFAULT_TESTING_INTERVAL,
            retry_timeout: DEFAULT_RETRY_TIMEOUT,
            validator_urls: vec![],
            mixnet_contract_address: "".to_string(),
        })
    }
}

pub struct VerlocMeasurer {
    config: Config,
    packet_sender: Arc<PacketSender>,
    packet_listener: Arc<PacketListener>,

    // Note: this client is only fine here as it does not maintain constant connection to the validator.
    // It only does bunch of REST queries. If we update it at some point to a more sophisticated (maybe signing) client,
    // then it definitely cannot be constructed here and probably will need to be passed from outside,
    // as mixnodes/gateways would already be using an instance of said client.
    validator_client: validator_client::Client,
    results: AtomicVerlocResult,
}

impl VerlocMeasurer {
    pub fn new(config: Config, identity: Arc<identity::KeyPair>) -> Self {
        VerlocMeasurer {
            packet_sender: Arc::new(PacketSender::new(
                Arc::clone(&identity),
                config.packets_per_node,
                config.packet_timeout,
                config.connection_timeout,
                config.delay_between_packets,
            )),
            packet_listener: Arc::new(PacketListener::new(
                config.listening_address,
                Arc::clone(&identity),
            )),
            validator_client: validator_client::Client::new(validator_client::Config::new(
                config.validator_urls.clone(),
                config.mixnet_contract_address.clone(),
            )),
            config,
            results: AtomicVerlocResult::new(),
        }
    }

    pub fn get_verloc_results_pointer(&self) -> AtomicVerlocResult {
        self.results.clone_data_pointer()
    }

    fn start_listening(&self) -> JoinHandle<()> {
        let packet_listener = Arc::clone(&self.packet_listener);
        tokio::spawn(packet_listener.run())
    }

    async fn perform_measurement(&self, nodes_to_test: Vec<TestedNode>) {
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
            while let Some(result) = measurement_chunk.next().await {
                // if we receive JoinError it means the task failed to get executed, so either there's a bigger issue with tokio
                // or there was a panic inside the task itself. In either case, we should just terminate ourselves.
                let execution_result = result.expect("the measurement task panicked!");
                let measurement_result = match execution_result.0 {
                    Err(err) => {
                        debug!(
                            "Failed to perform measurement for {} - {}",
                            execution_result.1.to_base58_string(),
                            err
                        );
                        None
                    }
                    Ok(result) => Some(result),
                };
                chunk_results.push(Verloc::new(execution_result.1, measurement_result));
            }

            // update the results vector with chunks as they become available (by default every 50 nodes)
            self.results.append_results(chunk_results).await;
        }
    }

    pub async fn run(&mut self) {
        self.start_listening();
        loop {
            info!(target: "verloc", "Starting verloc measurements");
            // TODO: should we also measure gateways?
            let all_mixes = match self.validator_client.get_cached_mix_nodes().await {
                Ok(nodes) => nodes,
                Err(err) => {
                    error!(
                        "failed to obtain list of mixnodes from the validator - {}",
                        err
                    );
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
                    // check if the node has sufficient version to be able to understand the packets
                    let node_version = parse_version(&node.mix_node.version).ok()?;
                    if node_version < self.config.minimum_compatible_node_version {
                        return None;
                    }

                    // try to parse the identity and host
                    let node_identity =
                        identity::PublicKey::from_base58_string(node.mix_node.identity_key).ok()?;

                    let verloc_host = (&*node.mix_node.host, node.mix_node.verloc_port)
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
            self.results.reset_results(tested_nodes.len()).await;

            self.perform_measurement(tested_nodes).await;

            // write current time to "run finished" field
            self.results.finish_measurements().await;

            info!(target: "verloc", "Finished performing verloc measurements. The next one will happen in {:?}", self.config.testing_interval);
            sleep(self.config.testing_interval).await
        }
    }
}

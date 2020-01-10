use crate::validator::config;
use directory_client::requests::presence_topology_get::PresenceTopologyGetRequester;
use directory_client::DirectoryClient;
use log::{debug, error, info, trace, warn};
use serde::export::fmt::Error;
use serde::export::Formatter;
use sphinx::route::{Node as SphinxNode, NodeAddressBytes};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use topology::{MixNode, MixProviderNode, NymTopology, NymTopologyError};

#[derive(Debug)]
pub enum HealthCheckerError {
    FailedToObtainTopologyError,
    InvalidTopologyError,
}

impl From<topology::NymTopologyError> for HealthCheckerError {
    fn from(_: NymTopologyError) -> Self {
        use HealthCheckerError::*;
        InvalidTopologyError
    }
}

#[derive(Debug)]
struct HealthCheckResult(Vec<NodeScore>);

impl std::fmt::Display for HealthCheckResult {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        write!(f, "NETWORK HEALTH\n==============\n")?;
        self.0
            .iter()
            .for_each(|score| write!(f, "{}\n", score).unwrap());
        Ok(())
    }
}

impl HealthCheckResult {
    fn zero_score<T: NymTopology>(topology: T) -> Self {
        warn!("The network is unhealthy, could not send any packets - returning zero score!");
        let mixes = topology.get_mix_nodes();
        let providers = topology.get_mix_provider_nodes();

        let health = mixes
            .into_iter()
            .map(|node| NodeScore::from_mixnode(node))
            .chain(
                providers
                    .into_iter()
                    .map(|node| NodeScore::from_provider(node)),
            )
            .collect();

        HealthCheckResult(health)
    }

    fn check_path(path: &Vec<SphinxNode>) -> bool {
        trace!("Checking path: {:?}", path);

        // TODO:
        true
    }

    pub fn calculate<T: NymTopology>(topology: T) -> Self {
        let all_paths = match topology.all_paths() {
            Ok(paths) => paths,
            Err(_) => return Self::zero_score(topology),
        };

        // create entries for all nodes
        let mut score_map = HashMap::new();
        topology.get_mix_nodes().into_iter().for_each(|node| {
            score_map.insert(
                NodeAddressBytes::from_b64_string(node.pub_key.clone()).0,
                NodeScore::from_mixnode(node),
            );
        });

        topology
            .get_mix_provider_nodes()
            .into_iter()
            .for_each(|node| {
                score_map.insert(
                    NodeAddressBytes::from_b64_string(node.pub_key.clone()).0,
                    NodeScore::from_provider(node),
                );
            });

        for path in all_paths {
            let path_status = HealthCheckResult::check_path(&path);
            for node in path {
                // if value doesn't exist, something extremely weird must have happened
                let current_score = score_map.get_mut(&node.pub_key.0);
                if current_score.is_none() {
                    return Self::zero_score(topology);
                }
                let current_score = current_score.unwrap();
                current_score.increase_packet_count(path_status);
            }
        }

        HealthCheckResult(score_map.drain().map(|(_, v)| v).collect())
    }
}

#[derive(Debug)]
struct NodeScore {
    pub_key: NodeAddressBytes,
    addresses: Vec<SocketAddr>,
    version: String,
    layer: String,
    packets_sent: u64,
    packets_received: u64,
}

impl NodeScore {
    fn from_mixnode(node: MixNode) -> Self {
        NodeScore {
            pub_key: NodeAddressBytes::from_b64_string(node.pub_key),
            addresses: vec![node.host],
            version: node.version,
            layer: format!("layer {}", node.layer),
            packets_sent: 0,
            packets_received: 0,
        }
    }

    fn from_provider(node: MixProviderNode) -> Self {
        NodeScore {
            pub_key: NodeAddressBytes::from_b64_string(node.pub_key),
            addresses: vec![node.mixnet_listener, node.client_listener],
            version: node.version,
            layer: format!("provider"),
            packets_sent: 0,
            packets_received: 0,
        }
    }

    fn increase_packet_count(&mut self, was_delivered: bool) {
        self.packets_sent += 1;
        if was_delivered {
            self.packets_received += 1;
        }
    }
}

impl std::fmt::Display for NodeScore {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        let fmtd_addresses = match self.addresses.len() {
            1 => format!("{}", self.addresses[0]),
            2 => format!("{}, {}", self.addresses[0], self.addresses[1]),
            n => {
                error!(
                    "could not format score - node has {} addresses while only 1 or 2 are allowed!",
                    n
                );
                return Err(std::fmt::Error);
            }
        };
        let health_percentage = match self.packets_sent {
            0 => 0.0,
            _ => (self.packets_received as f64 / self.packets_sent as f64) * 100.0,
        };
        let stringified_key = self.pub_key.to_b64_string();
        write!(
            f,
            "{}/{}\t({}%)\t|| {}\tv{} <{}> - {}",
            self.packets_received,
            self.packets_sent,
            health_percentage,
            self.layer,
            self.version,
            fmtd_addresses,
            stringified_key,
        )
    }
}

pub(crate) struct HealthChecker {
    directory_client: directory_client::Client,
    interval: Duration,
    num_test_packets: usize,
}

impl HealthChecker {
    pub fn new(config: config::HealthCheck) -> Self {
        debug!(
            "healthcheck will be using the following directory server: {:?}",
            config.directory_server
        );
        let directory_client_config = directory_client::Config::new(config.directory_server);
        HealthChecker {
            directory_client: directory_client::Client::new(directory_client_config),
            interval: Duration::from_secs_f64(config.interval),
            num_test_packets: config.num_test_packets,
        }
    }

    fn do_check(&self) -> Result<HealthCheckResult, HealthCheckerError> {
        trace!("going to perform a healthcheck!");
        let current_topology = match self.directory_client.presence_topology.get() {
            Ok(topology) => topology,
            Err(_) => return Err(HealthCheckerError::FailedToObtainTopologyError),
        };
        trace!("current topology: {:?}", current_topology);

        Ok(HealthCheckResult::calculate(current_topology))
    }

    pub async fn run(self) -> Result<(), HealthCheckerError> {
        debug!(
            "healthcheck will run every {:?} and will send {} packets to each node",
            self.interval, self.num_test_packets
        );

        loop {
            match self.do_check() {
                Ok(health) => info!("current network health: \n{}", health),
                Err(err) => error!("failed to perform healthcheck - {:?}", err),
            };

            tokio::time::delay_for(self.interval).await;
        }
    }
}

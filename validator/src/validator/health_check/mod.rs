use crate::validator::config;
use directory_client::requests::presence_topology_get::PresenceTopologyGetRequester;
use directory_client::DirectoryClient;
use log::{debug, error, info, trace, warn};
use serde::export::fmt::Error;
use serde::export::Formatter;
use sphinx::route::Node as SphinxNode;
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

    fn calculate<T: NymTopology>(topology: T) -> Self {
        HealthCheckResult(Vec::new())
    }
}

#[derive(Debug)]
struct NodeScore {
    pub_key: String,
    addresses: Vec<SocketAddr>,
    version: String,
    packets_sent: u64,
    packets_received: u64,
}

impl NodeScore {
    fn from_mixnode(node: MixNode) -> Self {
        NodeScore {
            pub_key: node.pub_key,
            addresses: vec![node.host],
            version: node.version,
            packets_sent: 0,
            packets_received: 0,
        }
    }

    fn from_provider(node: MixProviderNode) -> Self {
        NodeScore {
            pub_key: node.pub_key,
            addresses: vec![node.mixnet_listener, node.client_listener],
            version: node.version,
            packets_sent: 0,
            packets_received: 0,
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
        write!(
            f,
            "{}/{} ({}%) || v{} <{}> - {}",
            self.packets_received,
            self.packets_sent,
            health_percentage,
            self.version,
            fmtd_addresses,
            self.pub_key,
        )
    }
}

pub(crate) struct HealthChecker {
    directory_client: directory_client::Client,
    interval: Duration,
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
        }
    }

    fn check_path(path: Vec<SphinxNode>) -> bool {
        false
    }

    fn do_check(&self) -> Result<HealthCheckResult, HealthCheckerError> {
        trace!("going to perform a healthcheck!");
        let current_topology = match self.directory_client.presence_topology.get() {
            Ok(topology) => topology,
            Err(_) => return Err(HealthCheckerError::FailedToObtainTopologyError),
        };

        trace!("current topology: {:?}", current_topology);
        let all_paths = match current_topology.all_paths() {
            Ok(paths) => paths,
            Err(_) => return Ok(HealthCheckResult::zero_score(current_topology)),
        };

        Ok(HealthCheckResult::zero_score(current_topology))
    }

    pub async fn run(self) -> Result<(), HealthCheckerError> {
        debug!("healthcheck will run every {:?}", self.interval,);

        loop {
            match self.do_check() {
                Ok(health) => info!("current network health: \n{}", health),
                Err(err) => error!("failed to perform healthcheck - {:?}", err),
            };

            tokio::time::delay_for(self.interval).await;
        }
    }
}

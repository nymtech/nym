// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::{Config, Topology};
use crate::error::MixnodeError;
use crate::node::helpers::{load_identity_keys, load_sphinx_keys};
use crate::node::http::legacy::verloc::VerlocState;
use crate::node::http::HttpApiBuilder;
use crate::node::listener::connection_handler::packet_processing::PacketProcessor;
use crate::node::listener::connection_handler::ConnectionHandler;
use crate::node::listener::Listener;
use crate::node::node_description::NodeDescription;
use crate::node::node_statistics::SharedNodeStats;
use crate::node::packet_delayforwarder::{DelayForwarder, PacketDelayForwardSender};
use log::{error, info, warn};
use nym_bin_common::output_format::OutputFormat;
use nym_bin_common::version_checker::parse_version;
use nym_crypto::asymmetric::{encryption, identity};
use nym_mixnode_common::verloc::{self, AtomicVerlocResult, VerlocMeasurer};
use nym_task::{TaskClient, TaskManager};
use nym_topology::provider_trait::TopologyProvider;
use nym_topology_control::accessor::TopologyAccessor;
use nym_topology_control::nym_api_provider::NymApiTopologyProvider;
use nym_topology_control::TopologyRefresher;
use nym_topology_control::TopologyRefresherConfig;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::net::SocketAddr;
use std::process;
use std::sync::Arc;
use url::Url;

pub(crate) mod helpers;
mod http;
mod listener;
pub(crate) mod node_description;
mod node_statistics;
mod packet_delayforwarder;

// the MixNode will live for whole duration of this program
pub struct MixNode {
    config: Config,
    descriptor: NodeDescription,
    identity_keypair: Arc<identity::KeyPair>,
    sphinx_keypair: Arc<encryption::KeyPair>,
}

impl MixNode {
    pub fn new(config: Config) -> Result<Self, MixnodeError> {
        Ok(MixNode {
            descriptor: Self::load_node_description(&config),
            identity_keypair: Arc::new(load_identity_keys(&config)?),
            sphinx_keypair: Arc::new(load_sphinx_keys(&config)?),
            config,
        })
    }

    fn load_node_description(config: &Config) -> NodeDescription {
        NodeDescription::load_from_file(&config.storage_paths.node_description).unwrap_or_default()
    }

    /// Prints relevant node details to the console
    pub(crate) fn print_node_details(&self, output: OutputFormat) {
        let node_details = nym_types::mixnode::MixnodeNodeDetailsResponse {
            identity_key: self.identity_keypair.public_key().to_base58_string(),
            sphinx_key: self.sphinx_keypair.public_key().to_base58_string(),
            bind_address: self.config.mixnode.listening_address,
            version: self.config.mixnode.version.clone(),
            mix_port: self.config.mixnode.mix_port,
            http_api_port: self.config.http.bind_address.port(),
            verloc_port: self.config.mixnode.verloc_port,
        };

        println!("{}", output.format(&node_details));
    }

    fn start_http_api(
        &self,
        atomic_verloc_result: AtomicVerlocResult,
        node_stats_pointer: SharedNodeStats,
        task_client: TaskClient,
    ) -> Result<(), MixnodeError> {
        HttpApiBuilder::new(&self.config, &self.identity_keypair, &self.sphinx_keypair)
            .with_verloc(VerlocState::new(atomic_verloc_result))
            .with_mixing_stats(node_stats_pointer)
            .with_descriptor(self.descriptor.clone())
            .start(task_client)
    }

    fn start_node_stats_controller(
        &self,
        shutdown: TaskClient,
    ) -> (SharedNodeStats, node_statistics::UpdateSender) {
        info!("Starting node stats controller...");
        let controller = node_statistics::Controller::new(
            self.config.debug.node_stats_logging_delay,
            self.config.debug.node_stats_updating_delay,
            shutdown,
        );
        let node_stats_pointer = controller.get_node_stats_data_pointer();
        let update_sender = controller.start();

        (node_stats_pointer, update_sender)
    }

    fn start_socket_listener(
        &self,
        node_stats_update_sender: node_statistics::UpdateSender,
        delay_forwarding_channel: PacketDelayForwardSender,
        shutdown: TaskClient,
    ) {
        info!("Starting socket listener...");

        let packet_processor =
            PacketProcessor::new(self.sphinx_keypair.private_key(), node_stats_update_sender);

        let connection_handler = ConnectionHandler::new(packet_processor, delay_forwarding_channel);

        let listening_address = SocketAddr::new(
            self.config.mixnode.listening_address,
            self.config.mixnode.mix_port,
        );

        Listener::new(listening_address, shutdown).start(connection_handler);
    }

    fn start_packet_delay_forwarder(
        &mut self,
        node_stats_update_sender: node_statistics::UpdateSender,
        shutdown: TaskClient,
    ) -> PacketDelayForwardSender {
        info!("Starting packet delay-forwarder...");

        let client_config = nym_mixnet_client::Config::new(
            self.config.debug.packet_forwarding_initial_backoff,
            self.config.debug.packet_forwarding_maximum_backoff,
            self.config.debug.initial_connection_timeout,
            self.config.debug.maximum_connection_buffer_size,
            self.config.debug.use_legacy_framed_packet_version,
        );

        let mut packet_forwarder = DelayForwarder::new(
            nym_mixnet_client::Client::new(client_config),
            node_stats_update_sender,
            shutdown,
        );

        let packet_sender = packet_forwarder.sender();

        tokio::spawn(async move { packet_forwarder.run().await });
        packet_sender
    }

    fn start_verloc_measurements(&self, shutdown: TaskClient) -> AtomicVerlocResult {
        info!("Starting the round-trip-time measurer...");

        // this is a sanity check to make sure we didn't mess up with the minimum version at some point
        // and whether the user has run update if they're using old config
        // if this code exists in the node, it MUST BE compatible
        let config_version = parse_version(&self.config.mixnode.version)
            .expect("malformed version in the config file");
        let minimum_version = parse_version(verloc::MINIMUM_NODE_VERSION).unwrap();
        if config_version < minimum_version {
            error!("You seem to have not updated your mixnode configuration file - please run `upgrade` before attempting again");
            process::exit(1)
        }

        // use the same binding address with the HARDCODED port for time being (I don't like that approach personally)

        let listening_address = SocketAddr::new(
            self.config.mixnode.listening_address,
            self.config.mixnode.verloc_port,
        );

        let config = verloc::ConfigBuilder::new()
            .listening_address(listening_address)
            .packets_per_node(self.config.verloc.packets_per_node)
            .connection_timeout(self.config.verloc.connection_timeout)
            .packet_timeout(self.config.verloc.packet_timeout)
            .delay_between_packets(self.config.verloc.delay_between_packets)
            .tested_nodes_batch_size(self.config.verloc.tested_nodes_batch_size)
            .testing_interval(self.config.verloc.testing_interval)
            .retry_timeout(self.config.verloc.retry_timeout)
            .nym_api_urls(self.config.get_nym_api_endpoints())
            .build();

        let mut verloc_measurer =
            VerlocMeasurer::new(config, Arc::clone(&self.identity_keypair), shutdown);
        let atomic_verloc_results = verloc_measurer.get_verloc_results_pointer();
        tokio::spawn(async move { verloc_measurer.run().await });
        atomic_verloc_results
    }

    fn setup_topology_provider(nym_api_urls: Vec<Url>) -> Box<dyn TopologyProvider + Send + Sync> {
        // if no custom provider was ... provided ..., create one using nym-api
        Box::new(NymApiTopologyProvider::new(
            nym_api_urls,
            env!("CARGO_PKG_VERSION").to_string(),
        ))
    }

    // future responsible for periodically polling directory server and updating
    // the current global view of topology
    async fn start_topology_refresher(
        topology_provider: Box<dyn TopologyProvider + Send + Sync>,
        topology_config: Topology,
        topology_accessor: TopologyAccessor,
        mut shutdown: TaskClient,
    ) {
        let topology_refresher_config =
            TopologyRefresherConfig::new(topology_config.topology_refresh_rate);

        let mut topology_refresher = TopologyRefresher::new(
            topology_refresher_config,
            topology_accessor,
            topology_provider,
        );
        // before returning, block entire runtime to refresh the current network view so that any
        // components depending on topology would see a non-empty view
        info!("Obtaining initial network topology");
        topology_refresher.try_refresh().await;

        if topology_config.disable_refreshing {
            // if we're not spawning the refresher, don't cause shutdown immediately
            info!("The topology refesher is not going to be started");
            shutdown.mark_as_success();
        } else {
            // don't spawn the refresher if we don't want to be refreshing the topology.
            // only use the initial values obtained
            info!("Starting topology refresher...");
            topology_refresher.start_with_shutdown(shutdown);
        }
    }

    fn random_api_client(&self) -> nym_validator_client::NymApiClient {
        let endpoints = self.config.get_nym_api_endpoints();
        let nym_api = endpoints
            .choose(&mut thread_rng())
            .expect("The list of validator apis is empty");

        nym_validator_client::NymApiClient::new(nym_api.clone())
    }

    async fn check_if_bonded(&self) -> bool {
        // TODO: if anything, this should be getting data directly from the contract
        // as opposed to the validator API
        let validator_client = self.random_api_client();
        let existing_nodes = match validator_client.get_cached_mixnodes().await {
            Ok(nodes) => nodes,
            Err(err) => {
                error!(
                    "failed to grab initial network mixnodes - {err}\n \
                    Please try to startup again in few minutes",
                );
                process::exit(1);
            }
        };

        existing_nodes.iter().any(|node| {
            node.bond_information.mix_node.identity_key
                == self.identity_keypair.public_key().to_base58_string()
        })
    }

    async fn wait_for_interrupt(&self, mut shutdown: TaskManager) {
        let _res = shutdown.catch_interrupt().await;
        log::info!("Stopping nym mixnode");
    }

    pub async fn run(&mut self) -> Result<(), MixnodeError> {
        info!("Starting nym mixnode");

        if self.check_if_bonded().await {
            warn!("You seem to have bonded your mixnode before starting it - that's highly unrecommended as in the future it might result in slashing");
        }

        let shutdown = TaskManager::default();

        let (node_stats_pointer, node_stats_update_sender) = self
            .start_node_stats_controller(shutdown.subscribe().named("node_statistics::Controller"));

        let topology_provider = Self::setup_topology_provider(self.config.get_nym_api_endpoints());
        let shared_topology_access = TopologyAccessor::new();
        Self::start_topology_refresher(
            topology_provider,
            self.config.topology,
            shared_topology_access.clone(),
            shutdown.subscribe().named("TopologyRefresher"),
        )
        .await;
        let delay_forwarding_channel = self.start_packet_delay_forwarder(
            node_stats_update_sender.clone(),
            shutdown.subscribe().named("DelayForwarder"),
        );
        self.start_socket_listener(
            node_stats_update_sender,
            delay_forwarding_channel,
            shutdown.subscribe().named("Listener"),
        );
        let atomic_verloc_results =
            self.start_verloc_measurements(shutdown.subscribe().named("VerlocMeasurer"));

        // Rocket handles shutdown on it's own, but its shutdown handling should be incorporated
        // with that of the rest of the tasks.
        // Currently it's runtime is forcefully terminated once the mixnode exits.
        self.start_http_api(
            atomic_verloc_results,
            node_stats_pointer,
            shutdown.subscribe().named("http-api"),
        )?;

        info!("Finished nym mixnode startup procedure - it should now be able to receive mix traffic!");
        self.wait_for_interrupt(shutdown).await;
        Ok(())
    }
}

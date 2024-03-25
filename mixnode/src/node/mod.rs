// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::MixnodeError;
use crate::node::helpers::{load_identity_keys, load_sphinx_keys};
use crate::node::http::HttpApiBuilder;
use crate::node::listener::connection_handler::packet_processing::PacketProcessor;
use crate::node::listener::connection_handler::ConnectionHandler;
use crate::node::listener::Listener;
use crate::node::node_description::NodeDescription;
use crate::node::packet_delayforwarder::{DelayForwarder, PacketDelayForwardSender};
use log::{error, info, warn};
use nym_bin_common::output_format::OutputFormat;
use nym_crypto::asymmetric::{encryption, identity};
use nym_mixnode_common::verloc;
use nym_task::{TaskClient, TaskHandle};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::net::SocketAddr;
use std::process;
use std::sync::Arc;
use nym_mixnode_common::verloc::VerlocMeasurer;
use nym_node_http_api::state::metrics::{SharedMixingStats, SharedVerlocStats};

pub mod helpers;
mod http;
mod listener;
pub mod node_description;
mod node_statistics;
mod packet_delayforwarder;

// the MixNode will live for whole duration of this program
pub struct MixNode {
    config: Config,
    descriptor: NodeDescription,
    identity_keypair: Arc<identity::KeyPair>,
    sphinx_keypair: Arc<encryption::KeyPair>,

    run_http_server: bool,
    task_client: Option<TaskClient>,
    mixing_stats: Option<SharedMixingStats>,
    verloc_stats: Option<SharedVerlocStats>,
}

impl MixNode {
    pub fn new(config: Config) -> Result<Self, MixnodeError> {
        Ok(MixNode {
            run_http_server: false,
            descriptor: Self::load_node_description(&config),
            identity_keypair: Arc::new(load_identity_keys(&config)?),
            sphinx_keypair: Arc::new(load_sphinx_keys(&config)?),
            config,
            task_client: None,
            mixing_stats: None,
            verloc_stats: None,
        })
    }

    pub fn new_loaded(
        config: Config,
        descriptor: NodeDescription,
        identity_keypair: Arc<identity::KeyPair>,
        sphinx_keypair: Arc<encryption::KeyPair>,
    ) -> Self {
        MixNode {
            run_http_server: false,
            task_client: None,
            config,
            descriptor,
            identity_keypair,
            sphinx_keypair,
            mixing_stats: None,
            verloc_stats: None,
        }
    }

    pub fn disable_http_server(&mut self) {
        self.run_http_server = false
    }

    pub fn set_task_client(&mut self, task_client: TaskClient) {
        self.task_client = Some(task_client)
    }
    
    pub fn set_mixing_stats(&mut self, mixing_stats: SharedMixingStats) {
        self.mixing_stats = Some(mixing_stats);
    }
    
    pub fn set_verloc_stats(&mut self, verloc_stats: SharedVerlocStats) {
        self.verloc_stats = Some(verloc_stats)
    }

    fn load_node_description(config: &Config) -> NodeDescription {
        NodeDescription::load_from_file(&config.storage_paths.node_description).unwrap_or_default()
    }

    /// Prints relevant node details to the console
    pub fn print_node_details(&self, output: OutputFormat) {
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
        atomic_verloc_result: SharedVerlocStats,
        node_stats_pointer: SharedMixingStats,
        metrics_key: Option<&String>,
        task_client: TaskClient,
    ) -> Result<(), MixnodeError> {
        HttpApiBuilder::new(&self.config, &self.identity_keypair, &self.sphinx_keypair)
            .with_verloc(atomic_verloc_result)
            .with_mixing_stats(node_stats_pointer)
            .with_metrics_key(metrics_key)
            .with_descriptor(self.descriptor.clone())
            .start(task_client)
    }

    fn start_node_stats_controller(
        &mut self,
        shutdown: TaskClient,
    ) -> (SharedMixingStats, node_statistics::UpdateSender) {
        info!("Starting node stats controller...");
        let mixing_stats = self.mixing_stats.take().unwrap_or_default();
        
        let controller = node_statistics::Controller::new(
            self.config.debug.node_stats_logging_delay,
            self.config.debug.node_stats_updating_delay,
            mixing_stats.clone(),
            shutdown,
        );
        let update_sender = controller.start();

        (mixing_stats, update_sender)
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

    fn start_verloc_measurements(&mut self, shutdown: TaskClient) -> SharedVerlocStats {
        info!("Starting the round-trip-time measurer...");

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

        let verloc_state = self.verloc_stats.take().unwrap_or_default();
        let mut verloc_measurer =
            VerlocMeasurer::new(config, Arc::clone(&self.identity_keypair), shutdown);
        verloc_measurer.set_shared_state(verloc_state.clone());

        tokio::spawn(async move { verloc_measurer.run().await });
        verloc_state
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

    async fn wait_for_interrupt(&self, shutdown: TaskHandle) {
        let _res = shutdown.wait_for_shutdown().await;
        log::info!("Stopping nym mixnode");
    }

    pub async fn run(&mut self) -> Result<(), MixnodeError> {
        info!("Starting nym mixnode");

        if self.check_if_bonded().await {
            warn!("You seem to have bonded your mixnode before starting it - that's highly unrecommended as in the future it might result in slashing");
        }

        // Shutdown notifier for signalling tasks to stop
        let shutdown = self
            .task_client
            .take()
            .map(Into::<TaskHandle>::into)
            .unwrap_or_default()
            .name_if_unnamed("mixnode");

        let (node_stats_pointer, node_stats_update_sender) =
            self.start_node_stats_controller(shutdown.fork("node_statistics::Controller"));
        let delay_forwarding_channel = self.start_packet_delay_forwarder(
            node_stats_update_sender.clone(),
            shutdown.fork("DelayForwarder"),
        );
        self.start_socket_listener(
            node_stats_update_sender,
            delay_forwarding_channel,
            shutdown.fork("Listener"),
        );
        let atomic_verloc_results = self.start_verloc_measurements(shutdown.fork("VerlocMeasurer"));

        // Rocket handles shutdown on it's own, but its shutdown handling should be incorporated
        // with that of the rest of the tasks.
        // Currently it's runtime is forcefully terminated once the mixnode exits.
        if self.run_http_server {
            self.start_http_api(
                atomic_verloc_results,
                node_stats_pointer,
                self.config.metrics_key(),
                shutdown.fork("http-api"),
            )?;
        }

        info!("Finished nym mixnode startup procedure - it should now be able to receive mix traffic!");
        self.wait_for_interrupt(shutdown).await;
        
        Ok(())
    }
}

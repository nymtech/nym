// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::node::listener::connection_handler::packet_processing::PacketProcessor;
use crate::node::listener::connection_handler::ConnectionHandler;
use crate::node::listener::Listener;
use crate::node::packet_delayforwarder::{DelayForwarder, PacketDelayForwardSender};
use nym_crypto::asymmetric::{encryption, identity};
use nym_mixnode_common::verloc;
use nym_mixnode_common::verloc::VerlocMeasurer;
use nym_node_http_api::state::metrics::{SharedMixingStats, SharedVerlocStats};
use nym_task::{TaskClient, TaskHandle};
use std::net::SocketAddr;
use std::process;
use std::sync::Arc;
use tracing::{error, info, warn};

mod listener;
mod node_statistics;
mod packet_delayforwarder;

// the MixNode will live for whole duration of this program
pub struct MixNode {
    config: Config,
    identity_keypair: Arc<identity::KeyPair>,
    sphinx_keypair: Arc<encryption::KeyPair>,

    task_client: Option<TaskClient>,
    mixing_stats: Option<SharedMixingStats>,
    verloc_stats: Option<SharedVerlocStats>,
}

impl MixNode {
    pub fn new_loaded(
        config: Config,
        identity_keypair: Arc<identity::KeyPair>,
        sphinx_keypair: Arc<encryption::KeyPair>,
    ) -> Self {
        MixNode {
            task_client: None,
            config,
            identity_keypair,
            sphinx_keypair,
            mixing_stats: None,
            verloc_stats: None,
        }
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

    async fn check_if_bonded(&self) -> bool {
        // TODO: if anything, this should be getting data directly from the contract
        // as opposed to the validator API
        for api_url in self.config.get_nym_api_endpoints() {
            let client = nym_validator_client::NymApiClient::new(api_url.clone());
            match client.get_all_basic_nodes(None).await {
                Ok(nodes) => {
                    return nodes.iter().any(|node| {
                        &node.ed25519_identity_pubkey == self.identity_keypair.public_key()
                    })
                }
                Err(err) => {
                    error!("failed to grab initial network mixnodes from {api_url}: {err}",);
                }
            }
        }

        error!(
            "failed to grab initial network mixnodes from any of the available apis. Please try to startup again in few minutes",
        );
        process::exit(1);
    }

    async fn wait_for_interrupt(&self, shutdown: TaskHandle) {
        let _res = shutdown.wait_for_shutdown().await;
        info!("Stopping nym mixnode");
    }

    pub async fn run(&mut self) {
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

        let (_, node_stats_update_sender) =
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
        self.start_verloc_measurements(shutdown.fork("VerlocMeasurer"));

        info!("Finished nym mixnode startup procedure - it should now be able to receive mix traffic!");
        self.wait_for_interrupt(shutdown).await;
    }
}

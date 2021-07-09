// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::node::http::{
    description::description,
    not_found,
    stats::stats,
    verloc::{verloc as verlocRoute, VerlocState},
};
use crate::node::listener::connection_handler::packet_processing::PacketProcessor;
use crate::node::listener::connection_handler::ConnectionHandler;
use crate::node::listener::Listener;
use crate::node::node_description::NodeDescription;
use crate::node::node_statistics::NodeStatsWrapper;
use crate::node::packet_delayforwarder::{DelayForwarder, PacketDelayForwardSender};
use crypto::asymmetric::{encryption, identity};
use log::{error, info, warn};
use mixnode_common::verloc::{self, AtomicVerlocResult, VerlocMeasurer};
use std::net::SocketAddr;
use std::process;
use std::sync::Arc;
use tokio::runtime::Runtime;
use version_checker::parse_version;

pub(crate) mod http;
mod listener;
// mod metrics;
pub(crate) mod node_description;
pub(crate) mod node_statistics;
pub(crate) mod packet_delayforwarder;

// the MixNode will live for whole duration of this program
pub struct MixNode {
    config: Config,
    descriptor: NodeDescription,
    identity_keypair: Arc<identity::KeyPair>,
    sphinx_keypair: Arc<encryption::KeyPair>,
}

impl MixNode {
    pub fn new(
        config: Config,
        descriptor: NodeDescription,
        identity_keypair: identity::KeyPair,
        sphinx_keypair: encryption::KeyPair,
    ) -> Self {
        MixNode {
            config,
            descriptor,
            identity_keypair: Arc::new(identity_keypair),
            sphinx_keypair: Arc::new(sphinx_keypair),
        }
    }

    fn start_http_api(
        &self,
        atomic_verloc_result: AtomicVerlocResult,
        node_stats_pointer: NodeStatsWrapper,
    ) {
        info!("Starting HTTP API on http://localhost:8000");

        let mut config = rocket::config::Config::release_default();

        // bind to the same address as we are using for mixnodes
        config.address = self.config.get_listening_address();
        config.port = self.config.get_http_api_port();

        let verloc_state = VerlocState::new(atomic_verloc_result);
        let descriptor = self.descriptor.clone();

        tokio::spawn(async move {
            rocket::build()
                .configure(config)
                .mount("/", routes![verlocRoute, description, stats])
                .register("/", catchers![not_found])
                .manage(verloc_state)
                .manage(descriptor)
                .manage(node_stats_pointer)
                .launch()
                .await
        });
    }

    fn start_node_stats_controller(&self) -> (NodeStatsWrapper, node_statistics::UpdateSender) {
        info!("Starting node stats controller...");
        let controller = node_statistics::Controller::new(
            self.config.get_node_stats_logging_delay(),
            self.config.get_node_stats_updating_delay(),
        );
        let node_stats_pointer = controller.get_node_stats_data_pointer();
        let update_sender = controller.start();

        (node_stats_pointer, update_sender)
    }

    fn start_socket_listener(
        &self,
        node_stats_update_sender: node_statistics::UpdateSender,
        delay_forwarding_channel: PacketDelayForwardSender,
    ) {
        info!("Starting socket listener...");

        let packet_processor = PacketProcessor::new(
            self.sphinx_keypair.private_key(),
            node_stats_update_sender,
            self.config.get_cache_entry_ttl(),
        );

        let connection_handler = ConnectionHandler::new(packet_processor, delay_forwarding_channel);

        let listening_address = SocketAddr::new(
            self.config.get_listening_address(),
            self.config.get_mix_port(),
        );

        Listener::new(listening_address).start(connection_handler);
    }

    fn start_packet_delay_forwarder(
        &mut self,
        node_stats_update_sender: node_statistics::UpdateSender,
    ) -> PacketDelayForwardSender {
        info!("Starting packet delay-forwarder...");

        let mut packet_forwarder = DelayForwarder::new(
            self.config.get_packet_forwarding_initial_backoff(),
            self.config.get_packet_forwarding_maximum_backoff(),
            self.config.get_initial_connection_timeout(),
            self.config.get_maximum_connection_buffer_size(),
            node_stats_update_sender,
        );

        let packet_sender = packet_forwarder.sender();

        tokio::spawn(async move { packet_forwarder.run().await });
        packet_sender
    }

    fn start_verloc_measurements(&self) -> AtomicVerlocResult {
        info!("Starting the round-trip-time measurer...");

        // this is a sanity check to make sure we didn't mess up with the minimum version at some point
        // and whether the user has run update if they're using old config
        // if this code exists in the node, it MUST BE compatible
        let config_version =
            parse_version(self.config.get_version()).expect("malformed version in the config file");
        let minimum_version = parse_version(verloc::MINIMUM_NODE_VERSION).unwrap();
        if config_version < minimum_version {
            error!("You seem to have not updated your mixnode configuration file - please run `upgrade` before attempting again");
            process::exit(1)
        }

        // use the same binding address with the HARDCODED port for time being (I don't like that approach personally)

        let listening_address = SocketAddr::new(
            self.config.get_listening_address(),
            self.config.get_verloc_port(),
        );

        let config = verloc::ConfigBuilder::new()
            .listening_address(listening_address)
            .packets_per_node(self.config.get_measurement_packets_per_node())
            .connection_timeout(self.config.get_measurement_connection_timeout())
            .packet_timeout(self.config.get_measurement_packet_timeout())
            .delay_between_packets(self.config.get_measurement_delay_between_packets())
            .tested_nodes_batch_size(self.config.get_measurement_tested_nodes_batch_size())
            .testing_interval(self.config.get_measurement_testing_interval())
            .retry_timeout(self.config.get_measurement_retry_timeout())
            .validator_urls(self.config.get_validator_rest_endpoints())
            .mixnet_contract_address(self.config.get_validator_mixnet_contract_address())
            .build();

        let mut verloc_measurer = VerlocMeasurer::new(config, Arc::clone(&self.identity_keypair));
        let atomic_verloc_results = verloc_measurer.get_verloc_results_pointer();
        tokio::spawn(async move { verloc_measurer.run().await });
        atomic_verloc_results
    }

    // TODO: ask DH whether this function still makes sense in ^0.10
    async fn check_if_same_ip_node_exists(&mut self) -> Option<String> {
        let validator_client_config = validator_client::Config::new(
            self.config.get_validator_rest_endpoints(),
            self.config.get_validator_mixnet_contract_address(),
        );
        let validator_client = validator_client::Client::new(validator_client_config);

        let existing_nodes = match validator_client.get_cached_mix_nodes().await {
            Ok(nodes) => nodes,
            Err(err) => {
                error!("failed to grab initial network mixnodes - {}\n Please try to startup again in few minutes", err);
                process::exit(1);
            }
        };

        let our_host = self.config.get_announce_address();

        existing_nodes
            .iter()
            .find(|node| node.mix_node.host == our_host)
            .map(|node| node.mix_node.identity_key.clone())
    }

    async fn wait_for_interrupt(&self) {
        if let Err(e) = tokio::signal::ctrl_c().await {
            error!(
                "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
                e
            );
        }
        println!(
            "Received SIGINT - the mixnode will terminate now (threads are not yet nicely stopped, if you see stack traces that's alright)."
        );
    }

    pub fn run(&mut self) {
        info!("Starting nym mixnode");

        let runtime = Runtime::new().unwrap();

        runtime.block_on(async {
            if let Some(duplicate_node_key) = self.check_if_same_ip_node_exists().await {
                if duplicate_node_key == self.identity_keypair.public_key().to_base58_string() {
                    warn!("You seem to have bonded your mixnode before starting it - that's highly unrecommended as in the future it might result in slashing");
                } else {
                    log::error!(
                        "Our announce-host is identical to an existing node's announce-host! (its key is {:?})",
                        duplicate_node_key
                    );
                    return;
                }
            }

            let (node_stats_pointer, node_stats_update_sender) = self.start_node_stats_controller();
            let delay_forwarding_channel = self.start_packet_delay_forwarder(node_stats_update_sender.clone());
            self.start_socket_listener(node_stats_update_sender, delay_forwarding_channel);

            let atomic_verloc_results= self.start_verloc_measurements();
            self.start_http_api(atomic_verloc_results, node_stats_pointer);

            info!("Finished nym mixnode startup procedure - it should now be able to receive mix traffic!");
            self.wait_for_interrupt().await
        });
    }
}

// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::node::http::{
    description::description,
    not_found,
    verloc::{verloc, VerlocState},
};
use crate::node::listener::connection_handler::packet_processing::PacketProcessor;
use crate::node::listener::connection_handler::ConnectionHandler;
use crate::node::listener::Listener;
use crate::node::node_description::NodeDescription;
use crate::node::packet_delayforwarder::{DelayForwarder, PacketDelayForwardSender};
use crypto::asymmetric::{encryption, identity};
use log::{error, info, warn};
use mixnode_common::rtt_measurement::{self, AtomicVerlocResult, RttMeasurer};
use std::process;
use std::sync::Arc;
use tokio::runtime::Runtime;
use version_checker::parse_version;

pub(crate) mod http;
mod listener;
mod metrics;
pub(crate) mod node_description;
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

    fn start_http_api(&self, atomic_verloc_result: AtomicVerlocResult) {
        info!("Starting HTTP API on http://localhost:8000");

        let mut config = rocket::config::Config::release_default();
        // bind to the same address as we are using for mixnodes
        config.address = self.config.get_listening_address().ip();

        let verloc_state = VerlocState::new(atomic_verloc_result);
        let descriptor = self.descriptor.clone();

        tokio::spawn(async move {
            rocket::build()
                .configure(config)
                .mount("/", routes![verloc, description])
                .register("/", catchers![not_found])
                .manage(verloc_state)
                .manage(descriptor)
                .launch()
                .await
        });
    }

    fn start_metrics_reporter(&self) -> metrics::MetricsReporter {
        info!("Starting metrics reporter...");
        metrics::MetricsController::new(
            self.config.get_metrics_server(),
            self.identity_keypair.public_key().to_base58_string(),
            self.config.get_metrics_running_stats_logging_delay(),
        )
        .start()
    }

    fn start_socket_listener(
        &self,
        metrics_reporter: metrics::MetricsReporter,
        delay_forwarding_channel: PacketDelayForwardSender,
    ) {
        info!("Starting socket listener...");

        let packet_processor = PacketProcessor::new(
            self.sphinx_keypair.private_key(),
            metrics_reporter,
            self.config.get_cache_entry_ttl(),
        );

        let connection_handler = ConnectionHandler::new(packet_processor, delay_forwarding_channel);

        let listener = Listener::new(self.config.get_listening_address());

        listener.start(connection_handler);
    }

    fn start_packet_delay_forwarder(
        &mut self,
        metrics_reporter: metrics::MetricsReporter,
    ) -> PacketDelayForwardSender {
        info!("Starting packet delay-forwarder...");

        let mut packet_forwarder = DelayForwarder::new(
            self.config.get_packet_forwarding_initial_backoff(),
            self.config.get_packet_forwarding_maximum_backoff(),
            self.config.get_initial_connection_timeout(),
            self.config.get_maximum_connection_buffer_size(),
            metrics_reporter,
        );

        let packet_sender = packet_forwarder.sender();

        tokio::spawn(async move { packet_forwarder.run().await });
        packet_sender
    }

    fn start_rtt_measurer(&self) -> AtomicVerlocResult {
        info!("Starting the round-trip-time measurer...");

        // this is a sanity check to make sure we didn't mess up with the minimum version at some point
        // and whether the user has run update if they're using old config
        // if this code exists in the node, it MUST BE compatible
        let config_version =
            parse_version(self.config.get_version()).expect("malformed version in the config file");
        let minimum_version = parse_version(rtt_measurement::MINIMUM_NODE_VERSION).unwrap();
        if config_version < minimum_version {
            error!("You seem to have not updated your mixnode configuration file - please run `upgrade` before attempting again");
            process::exit(1)
        }

        // use the same binding address with the HARDCODED port for time being (I don't like that approach personally)

        let listening_address = rtt_measurement::replace_port(
            self.config.get_listening_address(),
            rtt_measurement::DEFAULT_MEASUREMENT_PORT,
        );
        let config = rtt_measurement::ConfigBuilder::new()
            .listening_address(listening_address)
            .packets_per_node(self.config.get_measurement_packets_per_node())
            .packet_timeout(self.config.get_measurement_packet_timeout())
            .delay_between_packets(self.config.get_measurement_delay_between_packets())
            .tested_nodes_batch_size(self.config.get_measurement_tested_nodes_batch_size())
            .testing_interval(self.config.get_measurement_testing_interval())
            .retry_timeout(self.config.get_measurement_retry_timeout())
            .validator_urls(self.config.get_validator_rest_endpoints())
            .mixnet_contract_address(self.config.get_validator_mixnet_contract_address())
            .build();

        let mut rtt_measurer = RttMeasurer::new(config, Arc::clone(&self.identity_keypair));
        let atomic_verloc_results = rtt_measurer.get_verloc_results_pointer();
        tokio::spawn(async move { rtt_measurer.run().await });
        atomic_verloc_results
    }

    // TODO: ask DH whether this function still makes sense in ^0.10
    async fn check_if_same_ip_node_exists(&mut self) -> Option<String> {
        let validator_client_config = validator_client::Config::new(
            self.config.get_validator_rest_endpoints(),
            self.config.get_validator_mixnet_contract_address(),
        );
        let mut validator_client = validator_client::Client::new(validator_client_config);

        let existing_nodes = match validator_client.get_mix_nodes().await {
            Ok(nodes) => nodes,
            Err(err) => {
                error!("failed to grab initial network mixnodes - {}\n Please try to startup again in few minutes", err);
                process::exit(1);
            }
        };

        existing_nodes
            .iter()
            .find(|node| node.mix_node.host == self.config.get_announce_address())
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
                    warn!("You seem to have bonded your mixnode before starting it - that's highly unrecommended as in the future it will result in slashing");
                } else {
                    log::error!(
                        "Our announce-host is identical to an existing node's announce-host! (its key is {:?})",
                        duplicate_node_key
                    );
                    return;
                }
            }

            let metrics_reporter = self.start_metrics_reporter();
            let delay_forwarding_channel = self.start_packet_delay_forwarder(metrics_reporter.clone());
            self.start_socket_listener(metrics_reporter, delay_forwarding_channel);

            let atomic_verloc_results= self.start_rtt_measurer();
            self.start_http_api(atomic_verloc_results);

            info!("Finished nym mixnode startup procedure - it should now be able to receive mix traffic!");
            self.wait_for_interrupt().await
        });
    }
}

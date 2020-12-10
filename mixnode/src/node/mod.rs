// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::node::listener::connection_handler::packet_processing::PacketProcessor;
use crate::node::listener::connection_handler::ConnectionHandler;
use crate::node::listener::Listener;
use crate::node::packet_delayforwarder::{DelayForwarder, PacketDelayForwardSender};
use crypto::asymmetric::{encryption, identity};
use log::*;
use std::sync::Arc;
use tokio::runtime::Runtime;

mod listener;
mod metrics;
pub(crate) mod packet_delayforwarder;
mod presence;

// the MixNode will live for whole duration of this program
pub struct MixNode {
    config: Config,
    identity_keypair: Arc<identity::KeyPair>,
    sphinx_keypair: Arc<encryption::KeyPair>,
}

impl MixNode {
    pub fn new(
        config: Config,
        identity_keypair: identity::KeyPair,
        sphinx_keypair: encryption::KeyPair,
    ) -> Self {
        MixNode {
            config,
            identity_keypair: Arc::new(identity_keypair),
            sphinx_keypair: Arc::new(sphinx_keypair),
        }
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

    async fn check_if_same_ip_node_exists(&mut self) -> Option<String> {
        let validator_client_config =
            validator_client::Config::new(self.config.get_validator_rest_endpoint());
        let validator_client = validator_client::Client::new(validator_client_config);
        let topology = validator_client
            .get_topology()
            .await
            .expect("failed to grab network topology");
        let existing_mixes_presence = topology.mix_nodes;
        existing_mixes_presence
            .iter()
            .find(|node| node.mix_host() == self.config.get_announce_address())
            .map(|node| node.identity())
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
        info!("Trying to unregister with the validator...");
        if let Err(err) = presence::unregister_with_validator(
            self.config.get_validator_rest_endpoint(),
            self.identity_keypair.public_key().to_base58_string(),
        )
        .await
        {
            error!("failed to unregister with validator... - {:?}", err)
        } else {
            info!("unregistration was successful!")
        }
    }

    pub fn run(&mut self) {
        info!("Starting nym mixnode");

        let mut runtime = Runtime::new().unwrap();

        runtime.block_on(async {
            if let Some(duplicate_node_key) = self.check_if_same_ip_node_exists().await {
                if duplicate_node_key == self.identity_keypair.public_key().to_base58_string() {
                    warn!("We seem to have not unregistered after going offline - there's a node with identical identity and announce-host as us registered.")
                } else {
                    error!(
                        "Our announce-host is identical to an existing node's announce-host! (its key is {:?}",
                        duplicate_node_key
                    );
                    return;
                }
            }

            if let Err(err) = presence::register_with_validator(
                self.config.get_validator_rest_endpoint(),
                self.config.get_announce_address(),
                self.identity_keypair.public_key().to_base58_string(),
                self.sphinx_keypair.public_key().to_base58_string(),
                self.config.get_version().to_string(),
                self.config.get_location(),
                self.config.get_layer(),
                self.config.get_incentives_address(),
            ).await {
                error!("failed to register with the validator - {:?}", err);
                return;
            }

            let metrics_reporter = self.start_metrics_reporter();
            let delay_forwarding_channel = self.start_packet_delay_forwarder(metrics_reporter.clone());
            self.start_socket_listener(metrics_reporter, delay_forwarding_channel);

            info!("Finished nym mixnode startup procedure - it should now be able to receive mix traffic!");

            self.wait_for_interrupt().await
        })
    }
}

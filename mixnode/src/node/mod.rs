// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::node::http::verloc::verloc;
use crate::node::listener::connection_handler::packet_processing::PacketProcessor;
use crate::node::listener::connection_handler::ConnectionHandler;
use crate::node::listener::Listener;
use crate::node::packet_delayforwarder::{DelayForwarder, PacketDelayForwardSender};
use crypto::asymmetric::{encryption, identity};
use log::{error, info, warn};
use std::process;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub(crate) mod http;
mod listener;
mod metrics;
pub(crate) mod packet_delayforwarder;

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

    fn start_http_api(&self) {
        info!("Starting HTTP API on port 8000...");
        tokio::spawn(async move { rocket::build().mount("/", routes![verloc]).launch().await });
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

    // TODO: ask DH whether this function still makes sense in ^0.10
    async fn check_if_same_ip_node_exists(&mut self) -> Option<String> {
        let validator_client_config = validator_client_rest::Config::new(
            self.config.get_validator_rest_endpoints(),
            self.config.get_validator_mixnet_contract_address(),
        );
        let mut validator_client = validator_client_rest::Client::new(validator_client_config);

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
            self.start_http_api();

            info!("Finished nym mixnode startup procedure - it should now be able to receive mix traffic!");
            self.wait_for_interrupt().await
        });
    }
}

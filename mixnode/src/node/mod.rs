// Copyright 2020 Nym Technologies SA
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

use crate::config::Config;
use crate::node::listener::connection_handler::packet_processing::PacketProcessor;
use crate::node::listener::connection_handler::ConnectionHandler;
use crate::node::listener::Listener;
use crypto::asymmetric::{encryption, identity};
use directory_client::DirectoryClient;
use log::*;
use mixnet_client::forwarder::{MixForwardingSender, PacketForwarder};
use std::sync::Arc;
use tokio::runtime::Runtime;

mod listener;
mod metrics;
mod presence;

// the MixNode will live for whole duration of this program
pub struct MixNode {
    config: Config,
    #[allow(dead_code)]
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

    fn start_presence_notifier(&self) {
        info!("Starting presence notifier...");
        let notifier_config = presence::NotifierConfig::new(
            self.config.get_location(),
            self.config.get_presence_directory_server(),
            self.config.get_announce_address(),
            self.sphinx_keypair.public_key().to_base58_string(),
            self.config.get_layer(),
            self.config.get_presence_sending_delay(),
        );
        presence::Notifier::new(notifier_config).start();
    }

    fn start_metrics_reporter(&self) -> metrics::MetricsReporter {
        info!("Starting metrics reporter...");
        metrics::MetricsController::new(
            self.config.get_metrics_directory_server(),
            self.sphinx_keypair.public_key().to_base58_string(),
            self.config.get_metrics_sending_delay(),
            self.config.get_metrics_running_stats_logging_delay(),
        )
        .start()
    }

    fn start_socket_listener(
        &self,
        metrics_reporter: metrics::MetricsReporter,
        forwarding_channel: MixForwardingSender,
    ) {
        info!("Starting socket listener...");

        let packet_processor = PacketProcessor::new(
            self.sphinx_keypair.private_key(),
            metrics_reporter,
            self.config.get_cache_entry_ttl(),
        );

        let connection_handler = ConnectionHandler::new(packet_processor, forwarding_channel);

        let listener = Listener::new(self.config.get_listening_address());

        listener.start(connection_handler);
    }

    fn start_packet_forwarder(&mut self) -> MixForwardingSender {
        info!("Starting packet forwarder...");

        let (mut packet_forwarder, packet_sender) = PacketForwarder::new(
            self.config.get_packet_forwarding_initial_backoff(),
            self.config.get_packet_forwarding_maximum_backoff(),
            self.config.get_initial_connection_timeout(),
        );

        tokio::spawn(async move { packet_forwarder.run().await });
        packet_sender
    }

    async fn check_if_same_ip_node_exists(&mut self) -> Option<String> {
        let directory_client_config =
            directory_client::Config::new(self.config.get_presence_directory_server());
        let directory_client = directory_client::Client::new(directory_client_config);
        let topology = directory_client.get_topology().await.ok()?;
        let existing_mixes_presence = topology.mix_nodes;
        existing_mixes_presence
            .iter()
            .find(|node| node.host == self.config.get_announce_address())
            .map(|node| node.pub_key.clone())
    }

    async fn wait_for_interrupt(&self) {
        if let Err(e) = tokio::signal::ctrl_c().await {
            error!(
                "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
                e
            );
        }
        println!(
            "Received SIGINT - the mixnode will terminate now (threads are not YET nicely stopped)"
        );
    }

    pub fn run(&mut self) {
        info!("Starting nym mixnode");
        let mut runtime = Runtime::new().unwrap();

        runtime.block_on(async {
            if let Some(duplicate_node_key) = self.check_if_same_ip_node_exists().await {
                error!(
                    "Our announce-host is identical to an existing node's announce-host! (its key is {:?}",
                    duplicate_node_key
                );
                return;
            }
            let forwarding_channel = self.start_packet_forwarder();
            let metrics_reporter = self.start_metrics_reporter();
            self.start_socket_listener(metrics_reporter, forwarding_channel);
            self.start_presence_notifier();

            info!("Finished nym mixnode startup procedure - it should now be able to receive mix traffic!");

            self.wait_for_interrupt().await
        })
    }
}

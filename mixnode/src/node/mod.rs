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
use crate::node::packet_processing::PacketProcessor;
use crypto::encryption;
use directory_client::presence::Topology;
use futures::channel::mpsc;
use log::*;
use std::net::SocketAddr;
use tokio::runtime::Runtime;
use topology::NymTopology;

mod listener;
mod metrics;
mod packet_forwarding;
pub(crate) mod packet_processing;
mod presence;

// the MixNode will live for whole duration of this program
pub struct MixNode {
    runtime: Runtime,
    config: Config,
    sphinx_keypair: encryption::KeyPair,
}

impl MixNode {
    pub fn new(config: Config, sphinx_keypair: encryption::KeyPair) -> Self {
        MixNode {
            runtime: Runtime::new().unwrap(),
            config,
            sphinx_keypair,
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
        presence::Notifier::new(notifier_config).start(self.runtime.handle());
    }

    fn start_metrics_reporter(&self) -> metrics::MetricsReporter {
        info!("Starting metrics reporter...");
        metrics::MetricsController::new(
            self.config.get_metrics_directory_server(),
            self.sphinx_keypair.public_key().to_base58_string(),
            self.config.get_metrics_sending_delay(),
            self.config.get_metrics_running_stats_logging_delay(),
        )
        .start(self.runtime.handle())
    }

    fn start_socket_listener(
        &self,
        metrics_reporter: metrics::MetricsReporter,
        forwarding_channel: mpsc::UnboundedSender<(SocketAddr, Vec<u8>)>,
    ) {
        info!("Starting socket listener...");
        // this is the only location where our private key is going to be copied
        // it will be held in memory owned by `MixNode` and inside an Arc of `PacketProcessor`
        let packet_processor =
            PacketProcessor::new(self.sphinx_keypair.private_key().clone(), metrics_reporter);

        listener::run_socket_listener(
            self.runtime.handle(),
            self.config.get_listening_address(),
            packet_processor,
            forwarding_channel,
        );
    }

    fn start_packet_forwarder(&mut self) -> mpsc::UnboundedSender<(SocketAddr, Vec<u8>)> {
        info!("Starting packet forwarder...");
        self.runtime
            .enter(|| {
                packet_forwarding::PacketForwarder::new(
                    self.config.get_packet_forwarding_initial_backoff(),
                    self.config.get_packet_forwarding_maximum_backoff(),
                    self.config.get_initial_connection_timeout(),
                )
            })
            .start(self.runtime.handle())
    }

    fn check_if_same_ip_node_exists(&mut self) -> Option<String> {
        // TODO: once we change to graph topology this here will need to be updated!
        let topology = self
            .runtime
            .block_on(Topology::new(self.config.get_presence_directory_server()));
        let existing_mixes_presence = topology.mix_nodes;
        existing_mixes_presence
            .iter()
            .find(|node| node.host == self.config.get_announce_address())
            .map(|node| node.pub_key.clone())
    }

    pub fn run(&mut self) {
        info!("Starting nym mixnode");

        if let Some(duplicate_node_key) = self.check_if_same_ip_node_exists() {
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

        if let Err(e) = self.runtime.block_on(tokio::signal::ctrl_c()) {
            error!(
                "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
                e
            );
        }

        println!(
            "Received SIGINT - the mixnode will terminate now (threads are not YET nicely stopped)"
        );
    }
}

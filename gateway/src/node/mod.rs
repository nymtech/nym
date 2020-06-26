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
use crate::node::client_handling::clients_handler::{ClientsHandler, ClientsHandlerRequestSender};
use crate::node::client_handling::websocket;
use crate::node::mixnet_handling::sender::{OutboundMixMessageSender, PacketForwarder};
use crate::node::storage::{inboxes, ClientLedger};
use crypto::asymmetric::{encryption, identity};
use directory_client::DirectoryClient;
use log::*;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub(crate) mod client_handling;
pub(crate) mod mixnet_handling;
mod presence;
pub(crate) mod storage;

pub struct Gateway {
    config: Config,
    /// ed25519 keypair used to assert one's identity.
    identity: Arc<identity::KeyPair>,
    /// x25519 keypair used for Diffie-Hellman. Currently only used for sphinx key derivation.
    encryption_keys: Arc<encryption::KeyPair>,
    registered_clients_ledger: ClientLedger,
    client_inbox_storage: inboxes::ClientStorage,
}

impl Gateway {
    pub fn new(
        config: Config,
        encryption_keys: encryption::KeyPair,
        identity: identity::KeyPair,
    ) -> Self {
        let registered_clients_ledger = match ClientLedger::load(config.get_clients_ledger_path()) {
            Err(e) => panic!(format!("Failed to load the ledger - {:?}", e)),
            Ok(ledger) => ledger,
        };
        let client_inbox_storage = inboxes::ClientStorage::new(
            config.get_message_retrieval_limit() as usize,
            config.get_stored_messages_filename_length(),
            config.get_clients_inboxes_dir(),
        );
        Gateway {
            config,
            identity: Arc::new(identity),
            encryption_keys: Arc::new(encryption_keys),
            client_inbox_storage,
            registered_clients_ledger,
        }
    }

    fn start_mix_socket_listener(
        &self,
        clients_handler_sender: ClientsHandlerRequestSender,
        ack_sender: OutboundMixMessageSender,
    ) {
        info!("Starting mix socket listener...");

        let packet_processor = mixnet_handling::PacketProcessor::new(
            Arc::clone(&self.encryption_keys),
            clients_handler_sender,
            self.client_inbox_storage.clone(),
            ack_sender,
        );

        mixnet_handling::Listener::new(self.config.get_mix_listening_address())
            .start(packet_processor);
    }

    fn start_client_websocket_listener(
        &self,
        forwarding_channel: OutboundMixMessageSender,
        clients_handler_sender: ClientsHandlerRequestSender,
    ) {
        info!("Starting client [web]socket listener...");

        websocket::Listener::new(
            self.config.get_clients_listening_address(),
            Arc::clone(&self.identity),
        )
        .start(clients_handler_sender, forwarding_channel);
    }

    fn start_packet_forwarder(&self) -> OutboundMixMessageSender {
        info!("Starting mix packet forwarder...");

        let (_, forwarding_channel) = PacketForwarder::new(
            self.config.get_packet_forwarding_initial_backoff(),
            self.config.get_packet_forwarding_maximum_backoff(),
            self.config.get_initial_connection_timeout(),
        )
        .start();
        forwarding_channel
    }

    fn start_clients_handler(&self) -> ClientsHandlerRequestSender {
        info!("Starting clients handler");
        let (_, clients_handler_sender) = ClientsHandler::new(
            self.registered_clients_ledger.clone(),
            self.client_inbox_storage.clone(),
        )
        .start();
        clients_handler_sender
    }

    fn start_presence_notifier(&self) {
        info!("Starting presence notifier...");
        let notifier_config = presence::NotifierConfig::new(
            self.config.get_location(),
            self.config.get_presence_directory_server(),
            self.config.get_mix_announce_address(),
            self.config.get_clients_announce_address(),
            self.identity.public_key().to_base58_string(),
            self.encryption_keys.public_key().to_base58_string(),
            self.config.get_presence_sending_delay(),
        );
        presence::Notifier::new(notifier_config, self.registered_clients_ledger.clone()).start();
    }

    async fn wait_for_interrupt(&self) {
        if let Err(e) = tokio::signal::ctrl_c().await {
            error!(
                "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
                e
            );
        }
        println!(
            "Received SIGINT - the gateway will terminate now (threads are not YET nicely stopped)"
        );
    }

    async fn check_if_same_ip_gateway_exists(&self) -> Option<String> {
        let announced_mix_host = self.config.get_mix_announce_address();
        let announced_clients_host = self.config.get_clients_announce_address();
        let directory_client_cfg =
            directory_client::Config::new(self.config.get_presence_directory_server());
        let topology = directory_client::Client::new(directory_client_cfg)
            .get_topology()
            .await
            .expect("Failed to retrieve network topology");

        let existing_gateways = topology.gateway_nodes;
        existing_gateways
            .iter()
            .find(|node| {
                node.mixnet_listener == announced_mix_host
                    || node.client_listener == announced_clients_host
            })
            .map(|node| node.identity_key.clone())
    }

    // Rather than starting all futures with explicit `&Handle` argument, let's see how it works
    // out if we make it implicit using `tokio::spawn` inside Runtime context.
    // Basically more or less equivalent of using #[tokio::main] attribute.
    pub fn run(&mut self) {
        info!("Starting nym gateway!");
        let mut runtime = Runtime::new().unwrap();

        runtime.block_on(async {

            if let Some(duplicate_gateway_key) = self.check_if_same_ip_gateway_exists().await {
                error!(
                    "Our announce-host is identical to an existing node's announce-host! (its key is {:?}",
                    duplicate_gateway_key
                );
                return;
            }



            let mix_forwarding_channel = self.start_packet_forwarder();
            let clients_handler_sender = self.start_clients_handler();

            self.start_mix_socket_listener(clients_handler_sender.clone(), mix_forwarding_channel.clone());
            self.start_client_websocket_listener(mix_forwarding_channel, clients_handler_sender);

            self.start_presence_notifier();

            info!("Finished nym gateway startup procedure - it should now be able to receive mix and client traffic!");

            self.wait_for_interrupt().await
        });
    }
}

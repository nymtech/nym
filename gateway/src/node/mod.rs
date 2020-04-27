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
use crypto::encryption;
use log::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

pub(crate) mod client_handling;
pub(crate) mod mixnet_handling;
mod presence;
pub(crate) mod storage;

// current issues in this file:
// - two calls to `Arc::new(self.sphinx_keypair.private_key().clone()),` - basically 2 separate
// Arcs to the same underlying data (well, after a clone), so what it ends up resulting in is
// private key being in 3 different places in memory rather than in a single location.
// Does it affect performance? No, not really. Is it *SUPER* insecure? Also, not as much, because
// if somebody could read memory of the machine, they probably got better attack vectors.
// Should it get fixed? Probably. But it's very low priority for time being.

pub struct Gateway {
    config: Config,
    sphinx_keypair: encryption::KeyPair,
    registered_clients_ledger: ClientLedger,
    client_inbox_storage: inboxes::ClientStorage,
}

impl Gateway {
    // the constructor differs from mixnodes and providers in that it takes keys directly
    // as opposed to having `Self::load_sphinx_keys(cfg: &Config)` method. Let's see
    // how it works out, because I'm not sure which one would be "better", but when I think about it,
    // I kinda prefer to delegate having to load the keys to outside the gateway
    pub fn new(config: Config, sphinx_keypair: encryption::KeyPair) -> Self {
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
            sphinx_keypair,
            client_inbox_storage,
            registered_clients_ledger,
        }
    }

    fn start_mix_socket_listener(&self, clients_handler_sender: ClientsHandlerRequestSender) {
        info!("Starting mix socket listener...");

        let packet_processor = mixnet_handling::PacketProcessor::new(
            Arc::new(self.sphinx_keypair.private_key().clone()),
            clients_handler_sender,
            self.client_inbox_storage.clone(),
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

        websocket::Listener::new(self.config.get_clients_listening_address())
            .start(clients_handler_sender, forwarding_channel);
    }

    fn start_packet_forwarder(&self) -> OutboundMixMessageSender {
        // TODO: put those into configs
        let initial_reconnection_backoff = Duration::from_millis(10_000);
        let maximum_reconnection_backoff = Duration::from_millis(300_000);
        let initial_connection_timeout = Duration::from_millis(1500);

        info!("Starting mix packet forwarder...");

        let (_, forwarding_channel) = PacketForwarder::new(
            initial_reconnection_backoff,
            maximum_reconnection_backoff,
            initial_connection_timeout,
        )
        .start();
        forwarding_channel
    }

    fn start_clients_handler(&self) -> ClientsHandlerRequestSender {
        info!("Starting clients handler");
        let (_, clients_handler_sender) = ClientsHandler::new(
            Arc::new(self.sphinx_keypair.private_key().clone()),
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
            self.sphinx_keypair.public_key().to_base58_string(),
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

    // Rather than starting all futures with explicit `&Handle` argument, let's see how it works
    // out if we make it implicit using `tokio::spawn` inside Runtime context.
    // Basically more or less equivalent of using #[tokio::main] attribute.
    pub fn run(&mut self) {
        info!("Starting nym gateway!");
        let mut runtime = Runtime::new().unwrap();

        runtime.block_on(async {
            let mix_forwarding_channel = self.start_packet_forwarder();
            let clients_handler_sender = self.start_clients_handler();

            self.start_mix_socket_listener(clients_handler_sender.clone());
            self.start_client_websocket_listener(mix_forwarding_channel, clients_handler_sender);

            self.start_presence_notifier();

            info!("Finished nym gateway startup procedure - it should now be able to receive mix and client traffic!");

            self.wait_for_interrupt().await
        });
    }
}

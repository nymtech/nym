// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::node::client_handling::clients_handler::{ClientsHandler, ClientsHandlerRequestSender};
use crate::node::client_handling::websocket;
use crate::node::mixnet_handling::receiver::connection_handler::ConnectionHandler;
use crate::node::storage::{inboxes, ClientLedger};
use crypto::asymmetric::{encryption, identity};
use log::*;
use mixnet_client::forwarder::{MixForwardingSender, PacketForwarder};
use std::process;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub(crate) mod client_handling;
pub(crate) mod mixnet_handling;
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
            Err(e) => panic!("Failed to load the ledger - {:?}", e),
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
        ack_sender: MixForwardingSender,
    ) {
        info!("Starting mix socket listener...");

        let packet_processor = mixnet_handling::PacketProcessor::new(
            self.encryption_keys.private_key(),
            self.config.get_cache_entry_ttl(),
        );

        let connection_handler = ConnectionHandler::new(
            packet_processor,
            clients_handler_sender,
            self.client_inbox_storage.clone(),
            ack_sender,
        );

        let listener = mixnet_handling::Listener::new(self.config.get_mix_listening_address());

        listener.start(connection_handler);
    }

    fn start_client_websocket_listener(
        &self,
        forwarding_channel: MixForwardingSender,
        clients_handler_sender: ClientsHandlerRequestSender,
    ) {
        info!("Starting client [web]socket listener...");

        websocket::Listener::new(
            self.config.get_clients_listening_address(),
            Arc::clone(&self.identity),
        )
        .start(clients_handler_sender, forwarding_channel);
    }

    fn start_packet_forwarder(&self) -> MixForwardingSender {
        info!("Starting mix packet forwarder...");

        let (mut packet_forwarder, packet_sender) = PacketForwarder::new(
            self.config.get_packet_forwarding_initial_backoff(),
            self.config.get_packet_forwarding_maximum_backoff(),
            self.config.get_initial_connection_timeout(),
            self.config.get_maximum_connection_buffer_size(),
        );

        tokio::spawn(async move { packet_forwarder.run().await });
        packet_sender
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

    async fn wait_for_interrupt(&self) {
        if let Err(e) = tokio::signal::ctrl_c().await {
            error!(
                "There was an error while capturing SIGINT - {:?}. We will terminate regardless",
                e
            );
        }
        println!(
            "Received SIGINT - the gateway will terminate now (threads are not yet nicely stopped, if you see stack traces that's alright)."
        );
    }

    // TODO: ask DH whether this function still makes sense in ^0.10
    async fn check_if_same_ip_gateway_exists(&self) -> Option<String> {
        let announced_mix_host = self.config.get_mix_announce_address();
        let announced_clients_host = self.config.get_clients_announce_address();

        let validator_client_config = validator_client::Config::new(
            self.config.get_validator_rest_endpoints(),
            self.config.get_validator_mixnet_contract_address(),
        );
        let mut validator_client = validator_client::Client::new(validator_client_config);

        let existing_gateways = match validator_client.get_gateways().await {
            Ok(gateways) => gateways,
            Err(err) => {
                error!("failed to grab initial network gateways - {}\n Please try to startup again in few minutes", err);
                process::exit(1);
            }
        };

        existing_gateways
            .iter()
            .find(|node| {
                node.gateway.mix_host == announced_mix_host
                    || node.gateway.clients_host == announced_clients_host
            })
            .map(|node| node.gateway().identity_key.clone())
    }

    // Rather than starting all futures with explicit `&Handle` argument, let's see how it works
    // out if we make it implicit using `tokio::spawn` inside Runtime context.
    // Basically more or less equivalent of using #[tokio::main] attribute.
    pub fn run(&mut self) {
        info!("Starting nym gateway!");

        let runtime = Runtime::new().unwrap();

        runtime.block_on(async {
            if let Some(duplicate_node_key) = self.check_if_same_ip_gateway_exists().await {
                if duplicate_node_key == self.identity.public_key().to_base58_string() {
                    warn!("We seem to have not unregistered after going offline - there's a node with identical identity and announce-host as us registered.")
                } else {
                    error!(
                        "Our announce-host is identical to an existing node's announce-host! (its key is {:?})",
                        duplicate_node_key
                    );
                    return;
                }
            }

            let mix_forwarding_channel = self.start_packet_forwarder();
            let clients_handler_sender = self.start_clients_handler();

            self.start_mix_socket_listener(clients_handler_sender.clone(), mix_forwarding_channel.clone());
            self.start_client_websocket_listener(mix_forwarding_channel, clients_handler_sender);

            info!("Finished nym gateway startup procedure - it should now be able to receive mix and client traffic!");

            self.wait_for_interrupt().await
        });
    }
}

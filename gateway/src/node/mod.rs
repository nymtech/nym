// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::node::client_handling::active_clients::ActiveClientsStore;
use crate::node::client_handling::websocket;
use crate::node::mixnet_handling::receiver::connection_handler::ConnectionHandler;
use crate::node::storage::PersistentStorage;
use crypto::asymmetric::{encryption, identity};
use log::*;
use mixnet_client::forwarder::{MixForwardingSender, PacketForwarder};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::net::SocketAddr;
use std::process;
use std::sync::Arc;

#[cfg(feature = "coconut")]
use coconut_interface::VerificationKey;
#[cfg(feature = "coconut")]
use credentials::obtain_aggregate_verification_key;

pub(crate) mod client_handling;
pub(crate) mod mixnet_handling;
pub(crate) mod storage;

pub struct Gateway {
    config: Config,
    /// ed25519 keypair used to assert one's identity.
    identity: Arc<identity::KeyPair>,
    /// x25519 keypair used for Diffie-Hellman. Currently only used for sphinx key derivation.
    encryption_keys: Arc<encryption::KeyPair>,
    storage: PersistentStorage,
}

impl Gateway {
    async fn initialise_storage(config: &Config) -> PersistentStorage {
        let path = config.get_persistent_store_path();
        let retrieval_limit = config.get_message_retrieval_limit();
        match PersistentStorage::init(path, retrieval_limit).await {
            Err(err) => panic!("failed to initialise gateway storage - {}", err),
            Ok(storage) => storage,
        }
    }

    pub async fn new(
        config: Config,
        encryption_keys: encryption::KeyPair,
        identity: identity::KeyPair,
    ) -> Self {
        let storage = Self::initialise_storage(&config).await;

        Gateway {
            config,
            identity: Arc::new(identity),
            encryption_keys: Arc::new(encryption_keys),
            storage,
        }
    }

    fn start_mix_socket_listener(
        &self,
        ack_sender: MixForwardingSender,
        active_clients_store: ActiveClientsStore,
    ) {
        info!("Starting mix socket listener...");

        let packet_processor =
            mixnet_handling::PacketProcessor::new(self.encryption_keys.private_key());

        let connection_handler = ConnectionHandler::new(
            packet_processor,
            self.storage.clone(),
            ack_sender,
            active_clients_store,
        );

        let listening_address = SocketAddr::new(
            self.config.get_listening_address(),
            self.config.get_mix_port(),
        );

        mixnet_handling::Listener::new(listening_address).start(connection_handler);
    }

    fn start_client_websocket_listener(
        &self,
        forwarding_channel: MixForwardingSender,
        active_clients_store: ActiveClientsStore,
        #[cfg(feature = "coconut")] verification_key: VerificationKey,
    ) {
        info!("Starting client [web]socket listener...");

        let listening_address = SocketAddr::new(
            self.config.get_listening_address(),
            self.config.get_clients_port(),
        );

        websocket::Listener::new(
            listening_address,
            Arc::clone(&self.identity),
            #[cfg(feature = "coconut")]
            verification_key,
            #[cfg(not(feature = "coconut"))]
            self.config.get_eth_endpoint(),
        )
        .start(
            forwarding_channel,
            self.storage.clone(),
            active_clients_store,
        );
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
        let endpoints = self.config.get_validator_api_endpoints();
        let validator_api = endpoints
            .choose(&mut thread_rng())
            .expect("The list of validator apis is empty");
        let validator_client = validator_client::ApiClient::new(validator_api.clone());

        let existing_gateways = match validator_client.get_cached_gateways().await {
            Ok(gateways) => gateways,
            Err(err) => {
                error!("failed to grab initial network gateways - {}\n Please try to startup again in few minutes", err);
                process::exit(1);
            }
        };

        let our_host = self.config.get_announce_address();

        existing_gateways
            .iter()
            .find(|node| node.gateway.host == our_host)
            .map(|node| node.gateway().identity_key.clone())
    }

    pub async fn run(&mut self) {
        info!("Starting nym gateway!");

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

        #[cfg(feature = "coconut")]
        let validators_verification_key =
            obtain_aggregate_verification_key(&self.config.get_validator_api_endpoints())
                .await
                .expect("failed to contact validators to obtain their verification keys");

        let mix_forwarding_channel = self.start_packet_forwarder();

        let active_clients_store = ActiveClientsStore::new();
        self.start_mix_socket_listener(
            mix_forwarding_channel.clone(),
            active_clients_store.clone(),
        );

        self.start_client_websocket_listener(
            mix_forwarding_channel,
            active_clients_store,
            #[cfg(feature = "coconut")]
            validators_verification_key,
        );

        info!("Finished nym gateway startup procedure - it should now be able to receive mix and client traffic!");

        self.wait_for_interrupt().await
    }
}

// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::commands::sign::load_identity_keys;
use crate::commands::validate_bech32_address_or_exit;
use crate::config::Config;
use crate::node::client_handling::active_clients::ActiveClientsStore;
use crate::node::client_handling::websocket;
use crate::node::mixnet_handling::receiver::connection_handler::ConnectionHandler;
use crate::node::storage::Storage;
use crypto::asymmetric::{encryption, identity};
use log::*;
use mixnet_client::forwarder::{MixForwardingSender, PacketForwarder};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::net::SocketAddr;
use std::process;
use std::sync::Arc;

use crate::config::persistence::pathfinder::GatewayPathfinder;
#[cfg(not(feature = "coconut"))]
use crate::node::client_handling::websocket::connection_handler::eth_events::ERC20Bridge;
#[cfg(feature = "coconut")]
use coconut_interface::VerificationKey;
#[cfg(feature = "coconut")]
use credentials::obtain_aggregate_verification_key;

use self::storage::PersistentStorage;

pub(crate) mod client_handling;
pub(crate) mod mixnet_handling;
pub(crate) mod storage;

/// Wire up and create Gateway instance
pub(crate) async fn create_gateway(config: Config) -> Gateway<PersistentStorage> {
    let storage = initialise_storage(&config).await;
    Gateway::new(config, storage).await
}

async fn initialise_storage(config: &Config) -> PersistentStorage {
    let path = config.get_persistent_store_path();
    let retrieval_limit = config.get_message_retrieval_limit();
    match PersistentStorage::init(path, retrieval_limit).await {
        Err(err) => panic!("failed to initialise gateway storage - {}", err),
        Ok(storage) => storage,
    }
}

pub(crate) struct Gateway<St: Storage> {
    config: Config,
    /// ed25519 keypair used to assert one's identity.
    identity_keypair: Arc<identity::KeyPair>,
    /// x25519 keypair used for Diffie-Hellman. Currently only used for sphinx key derivation.
    sphinx_keypair: Arc<encryption::KeyPair>,
    storage: St,
}

impl<St> Gateway<St>
where
    St: Storage + Clone + 'static,
{
    /// Construct from the given `Config` instance.
    pub async fn new(config: Config, storage: St) -> Self {
        let pathfinder = GatewayPathfinder::new_from_config(&config);
        // let storage = Self::initialise_storage(&config).await;

        Gateway {
            config,
            identity_keypair: Arc::new(Self::load_identity_keys(&pathfinder)),
            sphinx_keypair: Arc::new(Self::load_sphinx_keys(&pathfinder)),
            storage,
        }
    }

    #[cfg(test)]
    pub async fn new_from_keys_and_storage(
        config: Config,
        identity_keypair: identity::KeyPair,
        sphinx_keypair: encryption::KeyPair,
        storage: St,
    ) -> Self {
        Gateway {
            config,
            identity_keypair: Arc::new(identity_keypair),
            sphinx_keypair: Arc::new(sphinx_keypair),
            storage,
        }
    }

    fn load_identity_keys(pathfinder: &GatewayPathfinder) -> identity::KeyPair {
        let identity_keypair: identity::KeyPair =
            pemstore::load_keypair(&pemstore::KeyPairPath::new(
                pathfinder.private_identity_key().to_owned(),
                pathfinder.public_identity_key().to_owned(),
            ))
            .expect("Failed to read stored identity key files");
        identity_keypair
    }

    fn load_sphinx_keys(pathfinder: &GatewayPathfinder) -> encryption::KeyPair {
        let sphinx_keypair: encryption::KeyPair =
            pemstore::load_keypair(&pemstore::KeyPairPath::new(
                pathfinder.private_encryption_key().to_owned(),
                pathfinder.public_encryption_key().to_owned(),
            ))
            .expect("Failed to read stored sphinx key files");
        sphinx_keypair
    }

    /// Signs the node config's bech32 address to produce a verification code for use in the wallet.
    /// Exits if the address isn't valid (which should protect against manual edits).
    fn generate_owner_signature(&self) -> String {
        let pathfinder = GatewayPathfinder::new_from_config(&self.config);
        let identity_keypair = load_identity_keys(&pathfinder);
        let address = self.config.get_wallet_address();
        validate_bech32_address_or_exit(address);
        let verification_code = identity_keypair.private_key().sign_text(address);
        verification_code
    }

    pub(crate) fn print_node_details(&self) {
        println!(
            "Identity Key: {}",
            self.identity_keypair.public_key().to_base58_string()
        );
        println!(
            "Sphinx Key: {}",
            self.sphinx_keypair.public_key().to_base58_string()
        );
        println!("Owner Signature: {}", self.generate_owner_signature());
        println!(
            "Host: {} (bind address: {})",
            self.config.get_announce_address(),
            self.config.get_listening_address()
        );
        println!("Version: {}", self.config.get_version());
        println!(
            "Mix Port: {}, Clients port: {}",
            self.config.get_mix_port(),
            self.config.get_clients_port()
        );

        println!(
            "Data store is at: {:?}",
            self.config.get_persistent_store_path()
        );
    }

    fn start_mix_socket_listener(
        &self,
        ack_sender: MixForwardingSender,
        active_clients_store: ActiveClientsStore,
    ) {
        info!("Starting mix socket listener...");

        let packet_processor =
            mixnet_handling::PacketProcessor::new(self.sphinx_keypair.private_key());

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
        #[cfg(not(feature = "coconut"))] erc20_bridge: ERC20Bridge,
    ) {
        info!("Starting client [web]socket listener...");

        let listening_address = SocketAddr::new(
            self.config.get_listening_address(),
            self.config.get_clients_port(),
        );

        websocket::Listener::new(
            listening_address,
            Arc::clone(&self.identity_keypair),
            self.config.get_testnet_mode(),
            #[cfg(feature = "coconut")]
            verification_key,
            #[cfg(not(feature = "coconut"))]
            erc20_bridge,
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
            if duplicate_node_key == self.identity_keypair.public_key().to_base58_string() {
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

        #[cfg(not(feature = "coconut"))]
        let erc20_bridge = ERC20Bridge::new(
            self.config.get_eth_endpoint(),
            self.config.get_validator_nymd_endpoints(),
            self.config._get_cosmos_mnemonic(),
        );

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
            #[cfg(not(feature = "coconut"))]
            erc20_bridge,
        );

        info!("Finished nym gateway startup procedure - it should now be able to receive mix and client traffic!");

        self.wait_for_interrupt().await
    }
}

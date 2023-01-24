// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::storage::PersistentStorage;
use crate::commands::ensure_correct_bech32_prefix;
use crate::config::persistence::pathfinder::GatewayPathfinder;
use crate::config::Config;
use crate::error::GatewayError;
use crate::node::client_handling::active_clients::ActiveClientsStore;
use crate::node::client_handling::websocket;
use crate::node::client_handling::websocket::connection_handler::coconut::CoconutVerifier;
use crate::node::mixnet_handling::receiver::connection_handler::ConnectionHandler;
use crate::node::statistics::collector::GatewayStatisticsCollector;
use crate::node::storage::Storage;
use crate::{commands::sign::load_identity_keys, OutputFormat};
use colored::Colorize;
use log::*;
use mixnet_client::forwarder::{MixForwardingSender, PacketForwarder};
use network_defaults::NymNetworkDetails;
use nym_crypto::asymmetric::{encryption, identity};
use rand::seq::SliceRandom;
use rand::thread_rng;
use statistics_common::collector::StatisticsSender;
use std::error::Error;
use std::net::SocketAddr;
use std::process;
use std::sync::Arc;
use task::{TaskClient, TaskManager};
use validator_client::Client;

pub(crate) mod client_handling;
pub(crate) mod mixnet_handling;
pub(crate) mod statistics;
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
        Err(err) => panic!("failed to initialise gateway storage - {err}"),
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
            nym_pemstore::load_keypair(&nym_pemstore::KeyPairPath::new(
                pathfinder.private_identity_key().to_owned(),
                pathfinder.public_identity_key().to_owned(),
            ))
            .expect("Failed to read stored identity key files");
        identity_keypair
    }

    fn load_sphinx_keys(pathfinder: &GatewayPathfinder) -> encryption::KeyPair {
        let sphinx_keypair: encryption::KeyPair =
            nym_pemstore::load_keypair(&nym_pemstore::KeyPairPath::new(
                pathfinder.private_encryption_key().to_owned(),
                pathfinder.public_encryption_key().to_owned(),
            ))
            .expect("Failed to read stored sphinx key files");
        sphinx_keypair
    }

    /// Signs the node config's bech32 address to produce a verification code for use in the wallet.
    /// Exits if the address isn't valid (which should protect against manual edits).
    fn generate_owner_signature(&self) -> Result<String, GatewayError> {
        let pathfinder = GatewayPathfinder::new_from_config(&self.config);
        let identity_keypair = load_identity_keys(&pathfinder);
        let Some(address) = self.config.get_wallet_address() else {
            let error_message = "Error: gateway hasn't set its wallet address".red();
            eprintln!("{error_message}");
            eprintln!("Exiting...");
            process::exit(1);
        };
        // perform extra validation to ensure we have correct prefix
        ensure_correct_bech32_prefix(&address)?;
        let verification_code = identity_keypair.private_key().sign_text(address.as_ref());
        Ok(verification_code)
    }

    pub(crate) fn print_node_details(&self, output: OutputFormat) -> Result<(), GatewayError> {
        let node_details = nym_types::gateway::GatewayNodeDetailsResponse {
            identity_key: self.identity_keypair.public_key().to_base58_string(),
            sphinx_key: self.sphinx_keypair.public_key().to_base58_string(),
            owner_signature: self.generate_owner_signature()?,
            announce_address: self.config.get_announce_address(),
            bind_address: self.config.get_listening_address().to_string(),
            version: self.config.get_version().to_string(),
            mix_port: self.config.get_mix_port(),
            clients_port: self.config.get_clients_port(),
            data_store: self
                .config
                .get_persistent_store_path()
                .to_str()
                .unwrap_or(".")
                .to_string(),
        };

        match output {
            OutputFormat::Json => println!(
                "{}",
                serde_json::to_string(&node_details)
                    .unwrap_or_else(|_| "Could not serialize node details".to_string())
            ),
            OutputFormat::Text => println!("{}", node_details),
        }
        Ok(())
    }

    fn start_mix_socket_listener(
        &self,
        ack_sender: MixForwardingSender,
        active_clients_store: ActiveClientsStore,
        shutdown: TaskClient,
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

        mixnet_handling::Listener::new(listening_address, shutdown).start(connection_handler);
    }

    fn start_client_websocket_listener(
        &self,
        forwarding_channel: MixForwardingSender,
        active_clients_store: ActiveClientsStore,
        shutdown: TaskClient,
        coconut_verifier: Arc<CoconutVerifier>,
    ) {
        info!("Starting client [web]socket listener...");

        let listening_address = SocketAddr::new(
            self.config.get_listening_address(),
            self.config.get_clients_port(),
        );

        websocket::Listener::new(
            listening_address,
            Arc::clone(&self.identity_keypair),
            self.config.get_only_coconut_credentials(),
            coconut_verifier,
        )
        .start(
            forwarding_channel,
            self.storage.clone(),
            active_clients_store,
            shutdown,
        );
    }

    fn start_packet_forwarder(&self, shutdown: TaskClient) -> MixForwardingSender {
        info!("Starting mix packet forwarder...");

        let (mut packet_forwarder, packet_sender) = PacketForwarder::new(
            self.config.get_packet_forwarding_initial_backoff(),
            self.config.get_packet_forwarding_maximum_backoff(),
            self.config.get_initial_connection_timeout(),
            self.config.get_maximum_connection_buffer_size(),
            self.config.get_use_legacy_sphinx_framing(),
            shutdown,
        );

        tokio::spawn(async move { packet_forwarder.run().await });
        packet_sender
    }

    async fn wait_for_interrupt(
        &self,
        shutdown: TaskManager,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let res = shutdown.catch_interrupt().await;
        log::info!("Stopping nym gateway");
        res
    }

    fn random_api_client(&self) -> validator_client::NymApiClient {
        let endpoints = self.config.get_nym_api_endpoints();
        let nym_api = endpoints
            .choose(&mut thread_rng())
            .expect("The list of validator apis is empty");

        validator_client::NymApiClient::new(nym_api.clone())
    }

    fn random_nyxd_client(
        &self,
    ) -> validator_client::Client<validator_client::nyxd::SigningNyxdClient> {
        let endpoints = self.config.get_nyxd_urls();
        let validator_nyxd = endpoints
            .choose(&mut thread_rng())
            .expect("The list of validators is empty");

        let network_details = NymNetworkDetails::new_from_env();
        let client_config = validator_client::Config::try_from_nym_network_details(
            &network_details,
        )
        .expect("failed to construct valid validator client config with the provided network");

        let mut client = Client::new_signing(client_config, self.config.get_cosmos_mnemonic())
            .expect("Could not connect with mnemonic");
        client
            .change_nyxd(validator_nyxd.clone())
            .expect("Could not use the random nyxd URL");
        client
    }

    async fn check_if_same_ip_gateway_exists(&self) -> Result<Option<String>, GatewayError> {
        let validator_client = self.random_api_client();

        let existing_gateways = match validator_client.get_cached_gateways().await {
            Ok(gateways) => gateways,
            Err(err) => {
                error!("failed to grab initial network gateways - {err}\n Please try to startup again in few minutes");
                return Err(GatewayError::NetworkGatewaysQueryFailure { source: err });
            }
        };

        let our_host = self.config.get_announce_address();

        Ok(existing_gateways
            .iter()
            .find(|node| node.gateway.host == our_host)
            .map(|node| node.gateway().identity_key.clone()))
    }

    async fn ensure_no_duplicate_host_exists(&self) -> Result<(), GatewayError> {
        let local_identity = self.identity_keypair.public_key().to_base58_string();
        if let Some(remote_identity) = self.check_if_same_ip_gateway_exists().await? {
            if remote_identity == local_identity {
                warn!("We seem to have not unregistered after going offline - there's a node with identical identity and announce-host as us registered.")
            } else {
                error!(
                    "Our announce-host is identical to an existing node's announce-host! (its key is {remote_identity})",
                );
                return Err(GatewayError::DuplicateNodeHost {
                    host: self.config.get_announce_address(),
                    local_identity,
                    remote_identity,
                });
            }
        }
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Starting nym gateway!");

        self.ensure_no_duplicate_host_exists().await?;

        let shutdown = TaskManager::new(10);

        let coconut_verifier = {
            let nyxd_client = self.random_nyxd_client();
            CoconutVerifier::new(nyxd_client)
        };

        let mix_forwarding_channel = self.start_packet_forwarder(shutdown.subscribe());

        let active_clients_store = ActiveClientsStore::new();
        self.start_mix_socket_listener(
            mix_forwarding_channel.clone(),
            active_clients_store.clone(),
            shutdown.subscribe(),
        );

        if self.config.get_enabled_statistics() {
            let statistics_service_url = self.config.get_statistics_service_url();
            let stats_collector = GatewayStatisticsCollector::new(
                self.identity_keypair.public_key().to_base58_string(),
                active_clients_store.clone(),
                statistics_service_url,
            );
            let mut stats_sender = StatisticsSender::new(stats_collector);
            tokio::spawn(async move {
                stats_sender.run().await;
            });
        }

        self.start_client_websocket_listener(
            mix_forwarding_channel,
            active_clients_store,
            shutdown.subscribe(),
            Arc::new(coconut_verifier),
        );

        info!("Finished nym gateway startup procedure - it should now be able to receive mix and client traffic!");

        self.wait_for_interrupt(shutdown).await
    }
}

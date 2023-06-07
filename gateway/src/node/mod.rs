// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::storage::PersistentStorage;
use crate::config::persistence::paths::GatewayPaths;
use crate::config::Config;
use crate::error::GatewayError;
use crate::node::client_handling::active_clients::ActiveClientsStore;
use crate::node::client_handling::websocket;
use crate::node::client_handling::websocket::connection_handler::coconut::CoconutVerifier;
use crate::node::mixnet_handling::receiver::connection_handler::ConnectionHandler;
use crate::node::statistics::collector::GatewayStatisticsCollector;
use crate::node::storage::Storage;
use log::*;
use nym_bin_common::output_format::OutputFormat;
use nym_crypto::asymmetric::{encryption, identity};
use nym_mixnet_client::forwarder::{MixForwardingSender, PacketForwarder};
use nym_network_defaults::NymNetworkDetails;
use nym_statistics_common::collector::StatisticsSender;
use nym_task::{TaskClient, TaskManager};
use nym_validator_client::Client;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

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
    let path = &config.storage_paths.clients_storage;
    let retrieval_limit = config.debug.message_retrieval_limit;
    match PersistentStorage::init(path, retrieval_limit).await {
        Err(err) => panic!("failed to initialise gateway storage: {err}"),
        Ok(storage) => storage,
    }
}

pub(crate) struct Gateway<St> {
    config: Config,
    /// ed25519 keypair used to assert one's identity.
    identity_keypair: Arc<identity::KeyPair>,
    /// x25519 keypair used for Diffie-Hellman. Currently only used for sphinx key derivation.
    sphinx_keypair: Arc<encryption::KeyPair>,
    storage: St,
}

impl<St> Gateway<St> {
    /// Construct from the given `Config` instance.
    pub async fn new(config: Config, storage: St) -> Self {
        Gateway {
            storage,
            identity_keypair: Arc::new(Self::load_identity_keys(&config)),
            sphinx_keypair: Arc::new(Self::load_sphinx_keys(&config)),
            config,
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

    /// Loads identity keys stored on disk
    pub(crate) fn load_identity_keys(config: &Config) -> identity::KeyPair {
        let identity_keypair: identity::KeyPair =
            nym_pemstore::load_keypair(&nym_pemstore::KeyPairPath::new(
                config.storage_paths.keys.private_identity_key(),
                config.storage_paths.keys.public_identity_key(),
            ))
            .expect("Failed to read stored identity key files");
        identity_keypair
    }

    /// Loads Sphinx keys stored on disk
    fn load_sphinx_keys(config: &Config) -> encryption::KeyPair {
        let sphinx_keypair: encryption::KeyPair =
            nym_pemstore::load_keypair(&nym_pemstore::KeyPairPath::new(
                config.storage_paths.keys.private_encryption_key(),
                config.storage_paths.keys.public_encryption_key(),
            ))
            .expect("Failed to read stored sphinx key files");
        sphinx_keypair
    }

    pub(crate) fn print_node_details(&self, output: OutputFormat) {
        let node_details = nym_types::gateway::GatewayNodeDetailsResponse {
            identity_key: self.identity_keypair.public_key().to_base58_string(),
            sphinx_key: self.sphinx_keypair.public_key().to_base58_string(),
            bind_address: self.config.gateway.listening_address.to_string(),
            version: self.config.gateway.version.clone(),
            mix_port: self.config.gateway.mix_port,
            clients_port: self.config.gateway.clients_port,
            data_store: self.config.storage_paths.clients_storage.display().to_string(),
        };

        println!("{}", output.format(&node_details));
    }

    fn start_mix_socket_listener(
        &self,
        ack_sender: MixForwardingSender,
        active_clients_store: ActiveClientsStore,
        shutdown: TaskClient,
    ) where
        St: Storage + Clone + 'static,
    {
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
            self.config.gateway.listening_address,
            self.config.gateway.mix_port,
        );

        mixnet_handling::Listener::new(listening_address, shutdown).start(connection_handler);
    }

    fn start_client_websocket_listener(
        &self,
        forwarding_channel: MixForwardingSender,
        active_clients_store: ActiveClientsStore,
        shutdown: TaskClient,
        coconut_verifier: Arc<CoconutVerifier>,
    ) where
        St: Storage + Clone + 'static,
    {
        info!("Starting client [web]socket listener...");

        let listening_address = SocketAddr::new(
            self.config.gateway.listening_address,
            self.config.gateway.clients_port,
        );

        websocket::Listener::new(
            listening_address,
            Arc::clone(&self.identity_keypair),
            self.config.gateway.only_coconut_credentials,
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
            self.config.debug.packet_forwarding_initial_backoff,
            self.config.debug.packet_forwarding_maximum_backoff,
            self.config.debug.initial_connection_timeout,
            self.config.debug.maximum_connection_buffer_size,
            self.config.debug.use_legacy_framed_packet_version,
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

    fn random_api_client(&self) -> nym_validator_client::NymApiClient {
        let endpoints = self.config.get_nym_api_endpoints();
        let nym_api = endpoints
            .choose(&mut thread_rng())
            .expect("The list of validator apis is empty");

        nym_validator_client::NymApiClient::new(nym_api.clone())
    }

    fn random_nyxd_client(
        &self,
    ) -> nym_validator_client::Client<nym_validator_client::nyxd::DirectSigningNyxdClient> {
        let endpoints = self.config.get_nyxd_urls();
        let validator_nyxd = endpoints
            .choose(&mut thread_rng())
            .expect("The list of validators is empty");

        let network_details = NymNetworkDetails::new_from_env();
        let client_config = nym_validator_client::Config::try_from_nym_network_details(
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

    async fn check_if_bonded(&self) -> Result<bool, GatewayError> {
        // TODO: if anything, this should be getting data directly from the contract
        // as opposed to the validator API
        let validator_client = self.random_api_client();
        let existing_nodes = match validator_client.get_cached_gateways().await {
            Ok(nodes) => nodes,
            Err(err) => {
                error!("failed to grab initial network gateways - {err}\n Please try to startup again in few minutes");
                return Err(GatewayError::NetworkGatewaysQueryFailure { source: err });
            }
        };

        Ok(existing_nodes.iter().any(|node| {
            node.gateway.identity_key == self.identity_keypair.public_key().to_base58_string()
        }))
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error + Send + Sync>>
    where
        St: Storage + Clone + 'static,
    {
        info!("Starting nym gateway!");

        if self.check_if_bonded().await? {
            warn!("You seem to have bonded your gateway before starting it - that's highly unrecommended as in the future it might result in slashing");
        }

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

        if self.config.gateway.enabled_statistics {
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

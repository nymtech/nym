// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use self::storage::PersistentStorage;
use crate::config::Config;
use crate::error::GatewayError;
use crate::node::client_handling::active_clients::ActiveClientsStore;
use crate::node::client_handling::embedded_network_requester::{
    LocalNetworkRequesterHandle, MessageRouter,
};
use crate::node::client_handling::websocket;
use crate::node::client_handling::websocket::connection_handler::coconut::CoconutVerifier;
use crate::node::mixnet_handling::receiver::connection_handler::ConnectionHandler;
use crate::node::statistics::collector::GatewayStatisticsCollector;
use crate::node::storage::Storage;
use futures::channel::{mpsc, oneshot};
use log::*;
use nym_bin_common::output_format::OutputFormat;
use nym_crypto::asymmetric::{encryption, identity};
use nym_mixnet_client::forwarder::{MixForwardingSender, PacketForwarder};
use nym_network_defaults::NymNetworkDetails;
use nym_network_requester::{LocalGateway, NRServiceProviderBuilder};
use nym_pemstore::traits::PemStorableKeyPair;
use nym_pemstore::KeyPairPath;
use nym_statistics_common::collector::StatisticsSender;
use nym_task::{TaskClient, TaskManager};
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::error::Error;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(crate) mod client_handling;
pub(crate) mod mixnet_handling;
pub(crate) mod statistics;
pub(crate) mod storage;

/// Wire up and create Gateway instance
pub(crate) async fn create_gateway(
    config: Config,
    custom_nr_mixnet: Option<PathBuf>,
) -> Result<Gateway, GatewayError> {
    // don't attempt to read config if NR is disabled
    let network_requester_config = if config.network_requester.enabled {
        if let Some(path) = &config.storage_paths.network_requester_config {
            Some(load_network_requester_config(&config.gateway.id, path)?)
        } else {
            // if NR is enabled, the config path must be specified
            return Err(GatewayError::UnspecifiedNetworkRequesterConfig);
        }
    } else {
        None
    };

    let storage = initialise_main_storage(&config).await;

    let nr_opts = network_requester_config.map(|config| LocalNetworkRequesterOpts {
        config,
        custom_mixnet_path: custom_nr_mixnet,
    });

    Gateway::new(config, nr_opts, storage)
}

fn load_network_requester_config<P: AsRef<Path>>(
    id: &str,
    path: P,
) -> Result<nym_network_requester::Config, GatewayError> {
    let path = path.as_ref();
    nym_network_requester::Config::read_from_toml_file(path).map_err(|err| {
        GatewayError::NetworkRequesterConfigLoadFailure {
            id: id.to_string(),
            path: path.to_path_buf(),
            source: err,
        }
    })
}

async fn initialise_main_storage(config: &Config) -> PersistentStorage {
    let path = &config.storage_paths.clients_storage;
    let retrieval_limit = config.debug.message_retrieval_limit;
    match PersistentStorage::init(path, retrieval_limit).await {
        Err(err) => panic!("failed to initialise gateway storage: {err}"),
        Ok(storage) => storage,
    }
}

// TODO: refactor and move it elsewhere
#[derive(Debug, Clone)]
pub struct LocalNetworkRequesterOpts {
    config: nym_network_requester::Config,

    custom_mixnet_path: Option<PathBuf>,
}

pub(crate) struct Gateway<St = PersistentStorage> {
    config: Config,

    network_requester_opts: Option<LocalNetworkRequesterOpts>,

    /// ed25519 keypair used to assert one's identity.
    identity_keypair: Arc<identity::KeyPair>,
    /// x25519 keypair used for Diffie-Hellman. Currently only used for sphinx key derivation.
    sphinx_keypair: Arc<encryption::KeyPair>,
    storage: St,
}

impl<St> Gateway<St> {
    /// Construct from the given `Config` instance.
    pub fn new(
        config: Config,
        network_requester_opts: Option<LocalNetworkRequesterOpts>,
        storage: St,
    ) -> Result<Self, GatewayError> {
        Ok(Gateway {
            storage,
            identity_keypair: Arc::new(Self::load_identity_keys(&config)?),
            sphinx_keypair: Arc::new(Self::load_sphinx_keys(&config)?),
            config,
            network_requester_opts,
        })
    }

    #[cfg(test)]
    pub async fn new_from_keys_and_storage(
        config: Config,
        network_requester_opts: Option<LocalNetworkRequesterOpts>,
        identity_keypair: identity::KeyPair,
        sphinx_keypair: encryption::KeyPair,
        storage: St,
    ) -> Self {
        Gateway {
            config,
            network_requester_opts,
            identity_keypair: Arc::new(identity_keypair),
            sphinx_keypair: Arc::new(sphinx_keypair),
            storage,
        }
    }

    fn load_keypair<T: PemStorableKeyPair>(
        paths: KeyPairPath,
        name: impl Into<String>,
    ) -> Result<T, GatewayError> {
        nym_pemstore::load_keypair(&paths).map_err(|err| GatewayError::KeyPairLoadFailure {
            keys: name.into(),
            paths,
            err,
        })
    }

    /// Loads identity keys stored on disk
    pub(crate) fn load_identity_keys(config: &Config) -> Result<identity::KeyPair, GatewayError> {
        let identity_paths = KeyPairPath::new(
            config.storage_paths.keys.private_identity_key(),
            config.storage_paths.keys.public_identity_key(),
        );
        Self::load_keypair(identity_paths, "gateway identity keys")
    }

    /// Loads Sphinx keys stored on disk
    fn load_sphinx_keys(config: &Config) -> Result<encryption::KeyPair, GatewayError> {
        let sphinx_paths = KeyPairPath::new(
            config.storage_paths.keys.private_encryption_key(),
            config.storage_paths.keys.public_encryption_key(),
        );
        Self::load_keypair(sphinx_paths, "gateway sphinx keys")
    }

    pub(crate) fn print_node_details(&self, output: OutputFormat) {
        let node_details = nym_types::gateway::GatewayNodeDetailsResponse {
            identity_key: self.identity_keypair.public_key().to_base58_string(),
            sphinx_key: self.sphinx_keypair.public_key().to_base58_string(),
            bind_address: self.config.gateway.listening_address.to_string(),
            version: self.config.gateway.version.clone(),
            mix_port: self.config.gateway.mix_port,
            clients_port: self.config.gateway.clients_port,
            config_path: self
                .config
                .save_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            data_store: self
                .config
                .storage_paths
                .clients_storage
                .display()
                .to_string(),
            network_requester: None,
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

    // TODO: rethink the logic in this function...
    async fn maybe_start_network_requester(
        &self,
        forwarding_channel: MixForwardingSender,
        mut shutdown: TaskClient,
    ) -> Result<Option<LocalNetworkRequesterHandle>, GatewayError> {
        if !self.config.network_requester.enabled {
            info!("network requester is disabled");
            shutdown.mark_as_success();
            return Ok(None);
        } else {
            info!("Starting network requester...");
        }

        // if network requester is enabled, configuration file must be provided!
        let Some(nr_opts) = &self.network_requester_opts else {
            return Err(GatewayError::UnspecifiedNetworkRequesterConfig)
        };

        // TODO: one of many
        let mut nr_opts = nr_opts.clone();
        nr_opts.config.network_requester.open_proxy = true;
        nr_opts.config.base.set_high_default_traffic_volume();
        nr_opts.config.base.set_no_cover_traffic();
        nr_opts.config.base.set_no_per_hop_delays();

        // this gateway, whenever it has anything to send to its local NR will use fake_client_tx
        let (nr_mix_sender, nr_mix_receiver) = mpsc::unbounded();
        let router_shutdown = shutdown.fork("message_router");

        let (router_tx, mut router_rx) = oneshot::channel();

        let transceiver = LocalGateway::new(
            *self.identity_keypair.public_key(),
            forwarding_channel,
            router_tx,
        );

        // TODO: well, wire it up internally to gateway traffic, shutdowns, etc.
        let (on_start_tx, on_start_rx) = oneshot::channel();
        let mut nr_builder = NRServiceProviderBuilder::new(nr_opts.config.clone())
            .with_shutdown(shutdown)
            .with_custom_gateway_transceiver(Box::new(transceiver))
            .with_on_start(on_start_tx);

        if let Some(custom_mixnet) = &nr_opts.custom_mixnet_path {
            nr_builder = nr_builder.with_stored_topology(custom_mixnet)?
        }

        tokio::spawn(async move {
            if let Err(err) = nr_builder.run_service_provider().await {
                // no need to panic as we have passed a task client to the NR so we're most likely
                // already in the process of shutting down
                error!("network requester has failed: {err}")
            }
        });

        let start_data = on_start_rx
            .await
            .map_err(|_| GatewayError::NetworkRequesterStartupFailure)?;

        // this should be instantaneous since the data is sent on this channel before the on start is called;
        // the failure should be impossible
        let Ok(Some(packet_router)) = router_rx.try_recv() else {
            return Err(GatewayError::NetworkRequesterStartupFailure)
        };

        MessageRouter::new(nr_mix_receiver, packet_router).start_with_shutdown(router_shutdown);
        info!(
            "the local network requester is running on {}",
            start_data.address
        );

        Ok(Some(LocalNetworkRequesterHandle::new(
            start_data,
            nr_mix_sender,
        )))
    }

    async fn wait_for_interrupt(shutdown: TaskManager) -> Result<(), Box<dyn Error + Send + Sync>> {
        let res = shutdown.catch_interrupt().await;
        log::info!("Stopping nym gateway");
        res
    }

    fn random_api_client(&self) -> Result<nym_validator_client::NymApiClient, GatewayError> {
        let endpoints = self.config.get_nym_api_endpoints();
        let nym_api = endpoints
            .choose(&mut thread_rng())
            .ok_or(GatewayError::NoNymApisAvailable)?;

        Ok(nym_validator_client::NymApiClient::new(nym_api.clone()))
    }

    fn random_nyxd_client(&self) -> Result<DirectSigningHttpRpcNyxdClient, GatewayError> {
        let endpoints = self.config.get_nyxd_urls();
        let validator_nyxd = endpoints
            .choose(&mut thread_rng())
            .ok_or(GatewayError::NoNyxdAvailable)?;

        let network_details = NymNetworkDetails::new_from_env();
        let client_config = nyxd::Config::try_from_nym_network_details(&network_details)?;

        DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            client_config,
            validator_nyxd.as_ref(),
            self.config.get_cosmos_mnemonic(),
        )
        .map_err(Into::into)
    }

    async fn check_if_bonded(&self) -> Result<bool, GatewayError> {
        // TODO: if anything, this should be getting data directly from the contract
        // as opposed to the validator API
        let validator_client = self.random_api_client()?;
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
            let nyxd_client = self.random_nyxd_client()?;
            CoconutVerifier::new(nyxd_client)
        };

        let mix_forwarding_channel =
            self.start_packet_forwarder(shutdown.subscribe().named("PacketForwarder"));

        let active_clients_store = ActiveClientsStore::new();
        self.start_mix_socket_listener(
            mix_forwarding_channel.clone(),
            active_clients_store.clone(),
            shutdown.subscribe().named("mixnet_handling::Listener"),
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

        if let Some(local_nr) = self
            .maybe_start_network_requester(
                mix_forwarding_channel.clone(),
                shutdown.subscribe().named("NetworkRequester"),
            )
            .await?
        {
            // insert information about local NR to the active clients store
            active_clients_store.insert_embedded(local_nr)
        }

        self.start_client_websocket_listener(
            mix_forwarding_channel,
            active_clients_store,
            shutdown.subscribe().named("websocket::Listener"),
            Arc::new(coconut_verifier),
        );

        // Once this is a bit more mature, make this a commandline flag instead of a compile time
        // flag
        #[cfg(feature = "wireguard")]
        nym_wireguard::start_wg_listener(shutdown.subscribe()).await?;

        info!("Finished nym gateway startup procedure - it should now be able to receive mix and client traffic!");

        Self::wait_for_interrupt(shutdown).await
    }
}

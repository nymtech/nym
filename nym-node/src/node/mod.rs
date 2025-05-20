// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use self::helpers::load_x25519_wireguard_keypair;
use crate::config::helpers::gateway_tasks_config;
use crate::config::{
    Config, GatewayTasksConfig, NodeModes, ServiceProvidersConfig, Wireguard, DEFAULT_MIXNET_PORT,
};
use crate::error::{EntryGatewayError, NymNodeError, ServiceProvidersError};
use crate::node::description::{load_node_description, save_node_description};
use crate::node::helpers::{
    get_current_rotation_id, load_ed25519_identity_keypair, load_key, load_x25519_noise_keypair,
    store_ed25519_identity_keypair, store_key, store_keypair, store_x25519_noise_keypair,
    DisplayDetails,
};
use crate::node::http::api::api_requests;
use crate::node::http::helpers::system_info::get_system_info;
use crate::node::http::state::{AppState, StaticNodeInformation};
use crate::node::http::{HttpServerConfig, NymNodeHttpServer, NymNodeRouter};
use crate::node::key_rotation::active_keys::ActiveSphinxKeys;
use crate::node::key_rotation::controller::KeyRotationController;
use crate::node::key_rotation::manager::SphinxKeyManager;
use crate::node::metrics::aggregator::MetricsAggregator;
use crate::node::metrics::console_logger::ConsoleLogger;
use crate::node::metrics::handler::client_sessions::GatewaySessionStatsHandler;
use crate::node::metrics::handler::global_prometheus_updater::PrometheusGlobalNodeMetricsRegistryUpdater;
use crate::node::metrics::handler::legacy_packet_data::LegacyMixingStatsUpdater;
use crate::node::metrics::handler::mixnet_data_cleaner::MixnetMetricsCleaner;
use crate::node::metrics::handler::pending_egress_packets_updater::PendingEgressPacketsUpdater;
use crate::node::mixnet::packet_forwarding::PacketForwarder;
use crate::node::mixnet::shared::ProcessingConfig;
use crate::node::mixnet::SharedFinalHopData;
use crate::node::nym_apis_client::NymApisClient;
use crate::node::replay_protection::background_task::ReplayProtectionDiskFlush;
use crate::node::replay_protection::bloomfilter::ReplayProtectionBloomfilters;
use crate::node::replay_protection::manager::ReplayProtectionBloomfiltersManager;
use crate::node::routing_filter::{OpenFilter, RoutingFilter};
use crate::node::shared_network::{
    CachedNetwork, CachedTopologyProvider, LocalGatewayNode, NetworkRefresher,
};
use nym_bin_common::bin_info;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_gateway::node::{ActiveClientsStore, GatewayTasksBuilder};
use nym_mixnet_client::client::ActiveConnections;
use nym_mixnet_client::forwarder::MixForwardingSender;
use nym_network_requester::{
    set_active_gateway, setup_fs_gateways_storage, store_gateway_details, CustomGatewayDetails,
    GatewayDetails, GatewayRegistration,
};
use nym_node_metrics::events::MetricEventsSender;
use nym_node_metrics::NymNodeMetrics;
use nym_node_requests::api::v1::node::models::{AnnouncePorts, NodeDescription};
use nym_noise::config::{NoiseConfig, NoiseNetworkView};
use nym_noise_keys::VersionedNoiseKey;
use nym_sphinx_acknowledgements::AckKey;
use nym_sphinx_addressing::Recipient;
use nym_task::{ShutdownManager, ShutdownToken, TaskClient};
use nym_validator_client::UserAgent;
use nym_verloc::measurements::SharedVerlocStats;
use nym_verloc::{self, measurements::VerlocMeasurer};
use nym_wireguard::{peer_controller::PeerControlRequest, WireguardGatewayData};
use rand::rngs::OsRng;
use rand::{CryptoRng, RngCore};
use std::net::SocketAddr;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, trace};
use zeroize::Zeroizing;

pub mod bonding_information;
pub mod description;
pub mod helpers;
pub(crate) mod http;
pub(crate) mod key_rotation;
pub(crate) mod metrics;
pub(crate) mod mixnet;
mod nym_apis_client;
pub(crate) mod replay_protection;
mod routing_filter;
mod shared_network;

pub struct GatewayTasksData {
    mnemonic: Arc<Zeroizing<bip39::Mnemonic>>,
    client_storage: nym_gateway::node::GatewayStorage,
    stats_storage: nym_gateway::node::PersistentStatsStorage,
}

impl GatewayTasksData {
    pub fn initialise(
        config: &GatewayTasksConfig,
        custom_mnemonic: Option<Zeroizing<bip39::Mnemonic>>,
    ) -> Result<(), EntryGatewayError> {
        // SAFETY:
        // this unwrap is fine as 24 word count is a valid argument for generating entropy for a new bip39 mnemonic
        #[allow(clippy::unwrap_used)]
        let mnemonic = Arc::new(
            custom_mnemonic
                .unwrap_or_else(|| Zeroizing::new(bip39::Mnemonic::generate(24).unwrap())),
        );
        config.storage_paths.save_mnemonic_to_file(&mnemonic)?;

        Ok(())
    }

    async fn new(config: &GatewayTasksConfig) -> Result<GatewayTasksData, EntryGatewayError> {
        let client_storage = nym_gateway::node::GatewayStorage::init(
            &config.storage_paths.clients_storage,
            config.debug.message_retrieval_limit,
        )
        .await
        .map_err(nym_gateway::GatewayError::from)?;

        let stats_storage =
            nym_gateway::node::PersistentStatsStorage::init(&config.storage_paths.stats_storage)
                .await
                .map_err(nym_gateway::GatewayError::from)?;

        Ok(GatewayTasksData {
            mnemonic: Arc::new(config.storage_paths.load_mnemonic_from_file()?),
            client_storage,
            stats_storage,
        })
    }
}

pub struct ServiceProvidersData {
    // ideally we'd be storing all the keys here, but unfortunately due to how the service providers
    // are currently implemented, they will be loading the data themselves from the provided paths

    // those public keys are just convenience wrappers for http builder and details displayer
    nr_ed25519: ed25519::PublicKey,
    nr_x25519: x25519::PublicKey,

    ipr_ed25519: ed25519::PublicKey,
    ipr_x25519: x25519::PublicKey,

    // TODO: those should be moved to WG section
    auth_ed25519: ed25519::PublicKey,
    auth_x25519: x25519::PublicKey,
}

impl ServiceProvidersData {
    fn initialise_client_keys<R: RngCore + CryptoRng>(
        rng: &mut R,
        typ: &str,
        ed25519_paths: nym_pemstore::KeyPairPath,
        x25519_paths: nym_pemstore::KeyPairPath,
        ack_key_path: &Path,
    ) -> Result<(), ServiceProvidersError> {
        let ed25519_keys = ed25519::KeyPair::new(rng);
        let x25519_keys = x25519::KeyPair::new(rng);
        let aes128ctr_key = AckKey::new(rng);

        store_keypair(
            &ed25519_keys,
            &ed25519_paths,
            format!("{typ}-ed25519-identity"),
        )?;
        store_keypair(&x25519_keys, &x25519_paths, format!("{typ}-x25519-dh"))?;
        store_key(&aes128ctr_key, ack_key_path, format!("{typ}-ack-key"))?;

        Ok(())
    }

    async fn initialise_client_gateway_storage(
        storage_path: &Path,
        registration: &GatewayRegistration,
    ) -> Result<(), ServiceProvidersError> {
        // insert all required information into the gateways store
        // (I hate that we have to do it, but that's currently the simplest thing to do)
        let storage = setup_fs_gateways_storage(storage_path).await?;
        store_gateway_details(&storage, registration).await?;
        set_active_gateway(&storage, &registration.gateway_id().to_base58_string()).await?;
        Ok(())
    }

    pub async fn initialise_network_requester<R: RngCore + CryptoRng>(
        rng: &mut R,
        config: &ServiceProvidersConfig,
        registration: &GatewayRegistration,
    ) -> Result<(), ServiceProvidersError> {
        trace!("initialising network requester keys");
        Self::initialise_client_keys(
            rng,
            "network-requester",
            config
                .storage_paths
                .network_requester
                .ed25519_identity_storage_paths(),
            config
                .storage_paths
                .network_requester
                .x25519_diffie_hellman_storage_paths(),
            &config.storage_paths.network_requester.ack_key_file,
        )?;
        Self::initialise_client_gateway_storage(
            &config.storage_paths.network_requester.gateway_registrations,
            registration,
        )
        .await
    }

    pub async fn initialise_ip_packet_router_requester<R: RngCore + CryptoRng>(
        rng: &mut R,
        config: &ServiceProvidersConfig,
        registration: &GatewayRegistration,
    ) -> Result<(), ServiceProvidersError> {
        trace!("initialising ip packet router keys");
        Self::initialise_client_keys(
            rng,
            "ip-packet-router",
            config
                .storage_paths
                .ip_packet_router
                .ed25519_identity_storage_paths(),
            config
                .storage_paths
                .ip_packet_router
                .x25519_diffie_hellman_storage_paths(),
            &config.storage_paths.ip_packet_router.ack_key_file,
        )?;
        Self::initialise_client_gateway_storage(
            &config.storage_paths.ip_packet_router.gateway_registrations,
            registration,
        )
        .await
    }

    pub async fn initialise_authenticator<R: RngCore + CryptoRng>(
        rng: &mut R,
        config: &ServiceProvidersConfig,
        registration: &GatewayRegistration,
    ) -> Result<(), ServiceProvidersError> {
        trace!("initialising authenticator keys");
        Self::initialise_client_keys(
            rng,
            "authenticator",
            config
                .storage_paths
                .authenticator
                .ed25519_identity_storage_paths(),
            config
                .storage_paths
                .authenticator
                .x25519_diffie_hellman_storage_paths(),
            &config.storage_paths.authenticator.ack_key_file,
        )?;
        Self::initialise_client_gateway_storage(
            &config.storage_paths.authenticator.gateway_registrations,
            registration,
        )
        .await?;
        Ok(())
    }

    pub async fn initialise(
        config: &ServiceProvidersConfig,
        public_key: ed25519::PublicKey,
    ) -> Result<(), ServiceProvidersError> {
        // generate all the keys for NR, IPR and AUTH
        let mut rng = OsRng;

        let gateway_details = GatewayDetails::Custom(CustomGatewayDetails::new(public_key)).into();

        // NR:
        Self::initialise_network_requester(&mut rng, config, &gateway_details).await?;

        // IPR:
        Self::initialise_ip_packet_router_requester(&mut rng, config, &gateway_details).await?;

        // Authenticator
        Self::initialise_authenticator(&mut rng, config, &gateway_details).await?;

        Ok(())
    }

    fn new(config: &ServiceProvidersConfig) -> Result<ServiceProvidersData, ServiceProvidersError> {
        let nr_paths = &config.storage_paths.network_requester;
        let nr_ed25519 = load_key(
            &nr_paths.public_ed25519_identity_key_file,
            "network requester ed25519",
        )?;

        let nr_x25519 = load_key(
            &nr_paths.public_x25519_diffie_hellman_key_file,
            "network requester x25519",
        )?;

        let ipr_paths = &config.storage_paths.ip_packet_router;
        let ipr_ed25519 = load_key(
            &ipr_paths.public_ed25519_identity_key_file,
            "ip packet router ed25519",
        )?;

        let ipr_x25519 = load_key(
            &ipr_paths.public_x25519_diffie_hellman_key_file,
            "ip packet router x25519",
        )?;

        let auth_paths = &config.storage_paths.authenticator;
        let auth_ed25519 = load_key(
            &auth_paths.public_ed25519_identity_key_file,
            "authenticator ed25519",
        )?;

        let auth_x25519 = load_key(
            &auth_paths.public_x25519_diffie_hellman_key_file,
            "authenticator x25519",
        )?;

        Ok(ServiceProvidersData {
            nr_ed25519,
            nr_x25519,
            ipr_ed25519,
            ipr_x25519,
            auth_ed25519,
            auth_x25519,
        })
    }
}

pub struct WireguardData {
    inner: WireguardGatewayData,
    peer_rx: mpsc::Receiver<PeerControlRequest>,
}

impl WireguardData {
    pub(crate) fn new(config: &Wireguard) -> Result<Self, NymNodeError> {
        let (inner, peer_rx) = WireguardGatewayData::new(
            config.clone().into(),
            Arc::new(load_x25519_wireguard_keypair(
                &config.storage_paths.x25519_wireguard_storage_paths(),
            )?),
        );
        Ok(WireguardData { inner, peer_rx })
    }

    pub(crate) fn initialise(config: &Wireguard) -> Result<(), ServiceProvidersError> {
        let mut rng = OsRng;
        let x25519_keys = x25519::KeyPair::new(&mut rng);

        store_keypair(
            &x25519_keys,
            &config.storage_paths.x25519_wireguard_storage_paths(),
            "wg-x25519-dh",
        )?;

        Ok(())
    }
}

impl From<WireguardData> for nym_wireguard::WireguardData {
    fn from(value: WireguardData) -> Self {
        nym_wireguard::WireguardData {
            inner: value.inner,
            peer_rx: value.peer_rx,
        }
    }
}

pub(crate) struct NymNode {
    config: Config,
    accepted_operator_terms_and_conditions: bool,
    shutdown_manager: ShutdownManager,

    description: NodeDescription,

    metrics: NymNodeMetrics,

    verloc_stats: SharedVerlocStats,

    entry_gateway: GatewayTasksData,

    #[allow(dead_code)]
    service_providers: ServiceProvidersData,

    wireguard: Option<WireguardData>,

    ed25519_identity_keys: Arc<ed25519::KeyPair>,
    sphinx_key_manager: Option<SphinxKeyManager>,

    // to be used when noise is integrated
    #[allow(dead_code)]
    x25519_noise_keys: Arc<x25519::KeyPair>,
}

impl NymNode {
    pub(crate) async fn initialise(
        config: &Config,
        custom_mnemonic: Option<Zeroizing<bip39::Mnemonic>>,
    ) -> Result<(), NymNodeError> {
        debug!("initialising nym-node with id: {}", config.id);
        let mut rng = OsRng;

        // global initialisation
        let ed25519_identity_keys = ed25519::KeyPair::new(&mut rng);
        let x25519_noise_keys = x25519::KeyPair::new(&mut rng);
        let current_rotation_id =
            get_current_rotation_id(&config.mixnet.nym_api_urls, &config.mixnet.nyxd_urls).await?;
        let _ = SphinxKeyManager::initialise_new(
            &mut rng,
            current_rotation_id,
            &config.storage_paths.keys.primary_x25519_sphinx_key_file,
            &config.storage_paths.keys.secondary_x25519_sphinx_key_file,
        )?;

        trace!("attempting to store ed25519 identity keypair");
        store_ed25519_identity_keypair(
            &ed25519_identity_keys,
            &config.storage_paths.keys.ed25519_identity_storage_paths(),
        )?;

        trace!("attempting to store x25519 noise keypair");
        store_x25519_noise_keypair(
            &x25519_noise_keys,
            &config.storage_paths.keys.x25519_noise_storage_paths(),
        )?;

        trace!("creating description file");
        save_node_description(
            &config.storage_paths.description,
            &NodeDescription::default(),
        )?;

        // entry gateway initialisation
        GatewayTasksData::initialise(&config.gateway_tasks, custom_mnemonic)?;

        // service providers initialisation
        ServiceProvidersData::initialise(
            &config.service_providers,
            *ed25519_identity_keys.public_key(),
        )
        .await?;

        // wireguard initialisation
        WireguardData::initialise(&config.wireguard)?;

        config.save()
    }

    pub(crate) async fn new(config: Config) -> Result<Self, NymNodeError> {
        let wireguard_data = WireguardData::new(&config.wireguard)?;
        let current_rotation_id =
            get_current_rotation_id(&config.mixnet.nym_api_urls, &config.mixnet.nyxd_urls).await?;

        Ok(NymNode {
            ed25519_identity_keys: Arc::new(load_ed25519_identity_keypair(
                &config.storage_paths.keys.ed25519_identity_storage_paths(),
            )?),
            sphinx_key_manager: Some(SphinxKeyManager::try_load_or_regenerate(
                current_rotation_id,
                &config.storage_paths.keys.primary_x25519_sphinx_key_file,
                &config.storage_paths.keys.secondary_x25519_sphinx_key_file,
            )?),
            x25519_noise_keys: Arc::new(load_x25519_noise_keypair(
                &config.storage_paths.keys.x25519_noise_storage_paths(),
            )?),
            description: load_node_description(&config.storage_paths.description)?,
            metrics: NymNodeMetrics::new(),
            verloc_stats: Default::default(),
            entry_gateway: GatewayTasksData::new(&config.gateway_tasks).await?,
            service_providers: ServiceProvidersData::new(&config.service_providers)?,
            wireguard: Some(wireguard_data),
            config,
            accepted_operator_terms_and_conditions: false,
            shutdown_manager: ShutdownManager::new("NymNode")
                .with_legacy_task_manager()
                .with_default_shutdown_signals()
                .map_err(|source| NymNodeError::ShutdownSignalFailure { source })?,
        })
    }

    pub(crate) fn config(&self) -> &Config {
        &self.config
    }

    pub(crate) fn shutdown_token<S: Into<String>>(&self, child_suffix: S) -> ShutdownToken {
        self.shutdown_manager.clone_token(child_suffix)
    }

    pub(crate) fn with_accepted_operator_terms_and_conditions(
        mut self,
        accepted_operator_terms_and_conditions: bool,
    ) -> Self {
        self.accepted_operator_terms_and_conditions = accepted_operator_terms_and_conditions;
        self
    }

    fn exit_network_requester_address(&self) -> Recipient {
        Recipient::new(
            self.service_providers.nr_ed25519,
            self.service_providers.nr_x25519,
            *self.ed25519_identity_keys.public_key(),
        )
    }

    fn exit_ip_packet_router_address(&self) -> Recipient {
        Recipient::new(
            self.service_providers.ipr_ed25519,
            self.service_providers.ipr_x25519,
            *self.ed25519_identity_keys.public_key(),
        )
    }

    fn exit_authenticator_address(&self) -> Recipient {
        Recipient::new(
            self.service_providers.auth_ed25519,
            self.service_providers.auth_x25519,
            *self.ed25519_identity_keys.public_key(),
        )
    }

    fn x25519_wireguard_key(&self) -> Result<x25519::PublicKey, NymNodeError> {
        let wg_data = self
            .wireguard
            .as_ref()
            .ok_or(NymNodeError::WireguardDataUnavailable)?;

        Ok(*wg_data.inner.keypair().public_key())
    }

    pub(crate) fn display_details(&self) -> Result<DisplayDetails, NymNodeError> {
        let sphinx_keys = self.sphinx_keys()?;
        Ok(DisplayDetails {
            current_modes: self.config.modes,
            description: self.description.clone(),
            ed25519_identity_key: self.ed25519_identity_key().to_base58_string(),
            x25519_primary_sphinx_key: sphinx_keys.keys.primary().deref().into(),
            x25519_secondary_sphinx_key: sphinx_keys.keys.secondary().map(|g| g.deref().into()),
            x25519_noise_key: self.x25519_noise_key().to_base58_string(),
            x25519_wireguard_key: self.x25519_wireguard_key()?.to_base58_string(),
            exit_network_requester_address: self.exit_network_requester_address().to_string(),
            exit_ip_packet_router_address: self.exit_ip_packet_router_address().to_string(),
            exit_authenticator_address: self.exit_authenticator_address().to_string(),
        })
    }

    pub(crate) fn modes(&self) -> NodeModes {
        self.config.modes
    }

    pub(crate) fn ed25519_identity_key(&self) -> &ed25519::PublicKey {
        self.ed25519_identity_keys.public_key()
    }

    pub(crate) fn x25519_noise_key(&self) -> &x25519::PublicKey {
        self.x25519_noise_keys.public_key()
    }

    #[track_caller]
    pub(crate) fn active_sphinx_keys(&self) -> Result<ActiveSphinxKeys, NymNodeError> {
        Ok(self.sphinx_keys()?.keys.clone())
    }

    async fn build_network_refresher(&self) -> Result<NetworkRefresher, NymNodeError> {
        NetworkRefresher::initialise_new(
            self.config.debug.testnet,
            Self::user_agent(),
            self.config.mixnet.nym_api_urls.clone(),
            self.config.debug.topology_cache_ttl,
            self.config.debug.routing_nodes_check_interval,
            self.shutdown_manager.clone_token("network-refresher"),
        )
        .await
    }

    fn as_gateway_topology_node(&self) -> Result<LocalGatewayNode, NymNodeError> {
        let ip_addresses = self.config.host.public_ips.clone();

        let Some(ip) = ip_addresses.first() else {
            return Err(NymNodeError::NoPublicIps);
        };

        let mix_port = self
            .config
            .mixnet
            .announce_port
            .unwrap_or(DEFAULT_MIXNET_PORT);
        let mix_host = SocketAddr::new(*ip, mix_port);

        let clients_ws_port = self
            .config
            .gateway_tasks
            .announce_ws_port
            .unwrap_or(self.config.gateway_tasks.ws_bind_address.port());

        Ok(LocalGatewayNode {
            active_sphinx_keys: self.active_sphinx_keys()?.clone(),
            mix_host,
            identity_key: *self.ed25519_identity_key(),
            entry: nym_topology::EntryDetails {
                ip_addresses,
                clients_ws_port,
                hostname: self.config.host.hostname.clone(),
                clients_wss_port: self.config.gateway_tasks.announce_wss_port,
            },
        })
    }

    async fn start_gateway_tasks(
        &mut self,
        cached_network: CachedNetwork,
        metrics_sender: MetricEventsSender,
        active_clients_store: ActiveClientsStore,
        mix_packet_sender: MixForwardingSender,
        task_client: TaskClient,
    ) -> Result<(), NymNodeError> {
        let config = gateway_tasks_config(&self.config);

        let topology_provider = Box::new(CachedTopologyProvider::new(
            self.as_gateway_topology_node()?,
            cached_network,
            self.config.gateway_tasks.debug.minimum_mix_performance,
        ));

        let mut gateway_tasks_builder = GatewayTasksBuilder::new(
            config.gateway,
            self.ed25519_identity_keys.clone(),
            self.entry_gateway.client_storage.clone(),
            mix_packet_sender,
            metrics_sender,
            self.metrics.clone(),
            self.entry_gateway.mnemonic.clone(),
            task_client,
        );

        // if we're running in entry mode, start the websocket
        if self.modes().entry {
            info!(
                "starting the clients websocket... on {}",
                self.config.gateway_tasks.ws_bind_address
            );
            let websocket = gateway_tasks_builder
                .build_websocket_listener(active_clients_store.clone())
                .await?;
            websocket.start();
        } else {
            info!("node not running in entry mode: the websocket will remain closed");
        }

        // if we're running in exit mode, start the IPR and NR
        if self.modes().exit {
            info!("starting the exit service providers: NR + IPR");
            gateway_tasks_builder.set_network_requester_opts(config.nr_opts);
            gateway_tasks_builder.set_ip_packet_router_opts(config.ipr_opts);

            let exit_sps = gateway_tasks_builder.build_exit_service_providers(
                topology_provider.clone(),
                topology_provider.clone(),
            )?;

            // note, this has all the joinhandles for when we want to use joinset
            let (started_nr, started_ipr) = exit_sps.start_service_providers().await?;
            active_clients_store.insert_embedded(started_nr.handle);
            active_clients_store.insert_embedded(started_ipr.handle);
            info!("started NR at: {}", started_nr.on_start_data.address);
            info!("started IPR at: {}", started_ipr.on_start_data.address);
        } else {
            info!("node not running in exit mode: the exit service providers (NR + IPR) will remain unavailable");
        }

        // if we're running wireguard, start the authenticator
        // and the actual wireguard listener
        if self.config.wireguard.enabled {
            info!("starting the wireguard tasks: authenticator service provider + wireguard peer controller");

            gateway_tasks_builder.set_authenticator_opts(config.auth_opts);

            // that's incredibly nasty, but unfortunately to change it, would require some refactoring...
            let Some(wg_data) = self.wireguard.take() else {
                return Err(NymNodeError::WireguardDataUnavailable);
            };

            gateway_tasks_builder.set_wireguard_data(wg_data.into());

            let authenticator = gateway_tasks_builder
                .build_wireguard_authenticator(topology_provider)
                .await?;
            let started_authenticator = authenticator.start_service_provider().await?;
            active_clients_store.insert_embedded(started_authenticator.handle);

            info!(
                "started authenticator at: {}",
                started_authenticator.on_start_data.address
            );

            gateway_tasks_builder
                .try_start_wireguard()
                .await
                .map_err(NymNodeError::GatewayTasksStartupFailure)?;
        } else {
            info!("node not running with wireguard: authenticator service provider and wireguard will remain unavailable");
        }

        // start task for removing stale and un-retrieved client messages
        let stale_messages_cleaner = gateway_tasks_builder.build_stale_messages_cleaner();
        stale_messages_cleaner.start();

        Ok(())
    }

    pub(crate) async fn build_http_server(&self) -> Result<NymNodeHttpServer, NymNodeError> {
        let host_details = sign_host_details(
            &self.config,
            self.x25519_sphinx_keys.public_key(),
            &VersionedNoiseKey {
                version: nym_noise::LATEST_NOISE_VERSION,
                x25519_pubkey: *self.x25519_noise_keys.public_key(),
            },
            &self.ed25519_identity_keys,
        )?;

        let auxiliary_details = api_requests::v1::node::models::AuxiliaryDetails {
            location: self.config.host.location,
            announce_ports: AnnouncePorts {
                verloc_port: self.config.verloc.announce_port,
                mix_port: self.config.mixnet.announce_port,
            },
            accepted_operator_terms_and_conditions: self.accepted_operator_terms_and_conditions,
        };

        // mixnode info
        let mixnode_details = api_requests::v1::mixnode::models::Mixnode {};

        // entry gateway info
        let wireguard = if self.config.wireguard.enabled {
            Some(api_requests::v1::gateway::models::Wireguard {
                port: self.config.wireguard.announced_port,
                public_key: self.x25519_wireguard_key()?.to_string(),
            })
        } else {
            None
        };
        let mixnet_websockets = Some(api_requests::v1::gateway::models::WebSockets {
            ws_port: self
                .config
                .gateway_tasks
                .announce_ws_port
                .unwrap_or(self.config.gateway_tasks.ws_bind_address.port()),
            wss_port: self.config.gateway_tasks.announce_wss_port,
        });
        let gateway_details = api_requests::v1::gateway::models::Gateway {
            enforces_zk_nyms: self.config.gateway_tasks.enforce_zk_nyms,
            client_interfaces: api_requests::v1::gateway::models::ClientInterfaces {
                wireguard,
                mixnet_websockets,
            },
        };

        // exit gateway info
        let nr_details = api_requests::v1::network_requester::models::NetworkRequester {
            encoded_identity_key: self.service_providers.nr_ed25519.to_base58_string(),
            encoded_x25519_key: self.service_providers.nr_x25519.to_base58_string(),
            address: self.exit_network_requester_address().to_string(),
        };

        let ipr_details = api_requests::v1::ip_packet_router::models::IpPacketRouter {
            encoded_identity_key: self.service_providers.ipr_ed25519.to_base58_string(),
            encoded_x25519_key: self.service_providers.ipr_x25519.to_base58_string(),
            address: self.exit_ip_packet_router_address().to_string(),
        };

        let auth_details = api_requests::v1::authenticator::models::Authenticator {
            encoded_identity_key: self.service_providers.auth_ed25519.to_base58_string(),
            encoded_x25519_key: self.service_providers.auth_x25519.to_base58_string(),
            address: self.exit_authenticator_address().to_string(),
        };

        let exit_policy_details =
            api_requests::v1::network_requester::exit_policy::models::UsedExitPolicy {
                enabled: true,
                upstream_source: self
                    .config
                    .service_providers
                    .upstream_exit_policy_url
                    .to_string(),
                last_updated: 0,
                // TODO: this will require some refactoring to actually retrieve the data from the embedded providers
                policy: None,
            };

        let mut config = HttpServerConfig::new()
            .with_landing_page_assets(self.config.http.landing_page_assets_path.as_ref())
            .with_mixnode_details(mixnode_details)
            .with_gateway_details(gateway_details)
            .with_network_requester_details(nr_details)
            .with_ip_packet_router_details(ipr_details)
            .with_authenticator_details(auth_details)
            .with_used_exit_policy(exit_policy_details)
            .with_description(self.description.clone())
            .with_auxiliary_details(auxiliary_details)
            .with_prometheus_bearer_token(self.config.http.access_token.clone());

        if self.config.http.expose_system_info {
            config = config.with_system_info(get_system_info(
                self.config.http.expose_system_hardware,
                self.config.http.expose_crypto_hardware,
            ))
        }
        if self.config.modes.mixnode {
            config.api.v1_config.node.roles.mixnode_enabled = true;
        }

        if self.config.modes.entry {
            config.api.v1_config.node.roles.gateway_enabled = true
        }

        if self.config.modes.exit {
            config.api.v1_config.node.roles.network_requester_enabled = true;
            config.api.v1_config.node.roles.ip_packet_router_enabled = true;
        }

        let x25519_versioned_noise_key = if self.config.mixnet.debug.unsafe_disable_noise {
            None
        } else {
            Some(VersionedNoiseKey {
                supported_version: nym_noise::LATEST_NOISE_VERSION,
                x25519_pubkey: *self.x25519_noise_keys.public_key(),
            })
        };

        let app_state = AppState::new(
            StaticNodeInformation {
                ed25519_identity_keys: self.ed25519_identity_keys.clone(),
                x25519_versioned_noise_key,
                ip_addresses: self.config.host.public_ips.clone(),
                hostname: self.config.host.hostname.clone(),
            },
            self.active_sphinx_keys()?.clone(),
            self.metrics.clone(),
            self.verloc_stats.clone(),
            self.config.http.node_load_cache_ttl,
        );

        Ok(NymNodeRouter::new(config, app_state)
            .build_server(&self.config.http.bind_address)
            .await?)
    }

    fn user_agent() -> UserAgent {
        bin_info!().into()
    }

    async fn try_refresh_remote_nym_api_cache(
        &self,
        client: &NymApisClient,
    ) -> Result<(), NymNodeError> {
        info!("attempting to request described cache refresh from nym-api(s)...");

        client
            .broadcast_force_refresh(self.ed25519_identity_keys.private_key())
            .await;
        Ok(())
    }

    pub(crate) fn start_verloc_measurements(&self) {
        info!(
            "Starting the [verloc] round-trip-time measurer on {} ...",
            self.config.verloc.bind_address
        );

        let mut base_agent = Self::user_agent();
        base_agent.application = format!("{}-verloc", base_agent.application);
        let config = nym_verloc::measurements::ConfigBuilder::new(
            self.config.mixnet.nym_api_urls.clone(),
            base_agent,
        )
        .listening_address(self.config.verloc.bind_address)
        .packets_per_node(self.config.verloc.debug.packets_per_node)
        .connection_timeout(self.config.verloc.debug.connection_timeout)
        .packet_timeout(self.config.verloc.debug.packet_timeout)
        .delay_between_packets(self.config.verloc.debug.delay_between_packets)
        .tested_nodes_batch_size(self.config.verloc.debug.tested_nodes_batch_size)
        .testing_interval(self.config.verloc.debug.testing_interval)
        .retry_timeout(self.config.verloc.debug.retry_timeout)
        .build();

        let mut verloc_measurer = VerlocMeasurer::new(
            config,
            self.ed25519_identity_keys.clone(),
            self.shutdown_manager.clone_token("verloc"),
        );
        verloc_measurer.set_shared_state(self.verloc_stats.clone());
        tokio::spawn(async move { verloc_measurer.run().await });
    }

    pub(crate) fn setup_metrics_backend(
        &self,
        active_clients_store: ActiveClientsStore,
        active_egress_mixnet_connections: ActiveConnections,
        shutdown: ShutdownToken,
    ) -> MetricEventsSender {
        info!("setting up node metrics...");

        // aggregator (to listen for any metrics events)
        let mut metrics_aggregator = MetricsAggregator::new(
            self.config.metrics.debug.aggregator_update_rate,
            shutdown.clone_with_suffix("aggregator"),
        );

        // >>>> START: register all relevant handlers for custom events

        // legacy metrics updater on the deprecated endpoint
        metrics_aggregator.register_handler(
            LegacyMixingStatsUpdater::new(self.metrics.clone()),
            self.config.metrics.debug.legacy_mixing_metrics_update_rate,
        );

        // stats for gateway client sessions (websocket-related information)
        metrics_aggregator.register_handler(
            GatewaySessionStatsHandler::new(
                self.metrics.clone(),
                self.entry_gateway.stats_storage.clone(),
            ),
            self.config.metrics.debug.clients_sessions_update_rate,
        );

        // handler for periodically cleaning up stale recipient/sender data
        metrics_aggregator.register_handler(
            MixnetMetricsCleaner::new(self.metrics.clone()),
            self.config.metrics.debug.stale_mixnet_metrics_cleaner_rate,
        );

        // handler for updating the value of forward/final hop packets pending delivery
        metrics_aggregator.register_handler(
            PendingEgressPacketsUpdater::new(
                self.metrics.clone(),
                active_clients_store,
                active_egress_mixnet_connections,
            ),
            self.config.metrics.debug.pending_egress_packets_update_rate,
        );

        // handler for updating the prometheus registry from the global atomic metrics counters
        // such as number of packets received
        metrics_aggregator.register_handler(
            PrometheusGlobalNodeMetricsRegistryUpdater::new(self.metrics.clone()),
            self.config
                .metrics
                .debug
                .global_prometheus_counters_update_rate,
        );

        // handler for handling prometheus metrics events
        // metrics_aggregator.register_handler(PrometheusEventsHandler{}, None);

        // note: we're still measuring things such as number of mixed packets,
        // but since they're stored as atomic integers, they are incremented directly at source
        // rather than going through event pipeline
        // should we need custom mixnet events, we can add additional handler for that. that's not a problem

        // >>>> END: register all relevant handlers

        // console logger to preserve old mixnode functionalities
        if self.config.metrics.debug.log_stats_to_console {
            ConsoleLogger::new(
                self.config.metrics.debug.console_logging_update_interval,
                self.metrics.clone(),
                shutdown.clone_with_suffix("metrics-console-logger"),
            )
            .start();
        }

        let events_sender = metrics_aggregator.sender();

        // spawn the aggregator task
        metrics_aggregator.start();

        events_sender
    }

    pub(crate) async fn setup_replay_detection(
        &self,
    ) -> Result<ReplayProtectionBloomfiltersManager, NymNodeError> {
        if self.config.mixnet.replay_protection.debug.unsafe_disabled {
            return Ok(ReplayProtectionBloomfiltersManager::new_disabled(
                self.metrics.clone(),
            ));
        }

        // create the background task for the bloomfilter
        // to reset it and flush it to disk
        let sphinx_keys = self.sphinx_keys()?;
        let mut replay_detection_background = ReplayProtectionDiskFlush::new(
            &self.config,
            sphinx_keys.keys.primary_key_rotation_id(),
            sphinx_keys.keys.secondary_key_rotation_id(),
            self.metrics.clone(),
            self.shutdown_manager
                .clone_token("replay-detection-background-flush"),
        )
        .await?;

        let bloomfilters_manager = replay_detection_background.bloomfilters_manager();
        self.shutdown_manager
            .spawn(async move { replay_detection_background.run().await });
        Ok(bloomfilters_manager)
    }

    // I'm assuming this will be needed in other places, so it's explicitly extracted
    fn setup_nym_apis_client(&self) -> Result<NymApisClient, NymNodeError> {
        NymApisClient::new(
            &self.config.mixnet.nym_api_urls,
            self.shutdown_manager.clone_token("nym-apis-client"),
        )
    }

    #[track_caller]
    fn sphinx_keys(&self) -> Result<&SphinxKeyManager, NymNodeError> {
        self.sphinx_key_manager
            .as_ref()
            .ok_or(NymNodeError::ConsumedSphinxKeys)
    }

    fn take_managed_sphinx_keys(&mut self) -> Result<SphinxKeyManager, NymNodeError> {
        self.sphinx_key_manager
            .take()
            .ok_or(NymNodeError::ConsumedSphinxKeys)
    }

    pub(crate) async fn setup_key_rotation(
        &mut self,
        nym_apis_client: NymApisClient,
        replay_protection_manager: ReplayProtectionBloomfiltersManager,
    ) -> Result<(), NymNodeError> {
        let managed_keys = self.take_managed_sphinx_keys()?;
        let rotation_state = nym_apis_client.get_key_rotation_info().await?;

        let rotation_controller = KeyRotationController::new(
            &self.config,
            rotation_state.into(),
            nym_apis_client,
            replay_protection_manager,
            managed_keys,
            self.shutdown_manager.clone_token("key-rotation-controller"),
        );

        rotation_controller.start();
        Ok(())
    }

    pub(crate) async fn start_mixnet_listener<F>(
        &self,
        active_clients_store: &ActiveClientsStore,
        replay_protection_bloomfilter: ReplayProtectionBloomfilters,
        routing_filter: F,
        noise_config: NoiseConfig,
        shutdown: ShutdownToken,
    ) -> Result<(MixForwardingSender, ActiveConnections), NymNodeError>
    where
        F: RoutingFilter + Send + Sync + 'static,
    {
        let processing_config = ProcessingConfig::new(&self.config);

        // we're ALWAYS listening for mixnet packets, either for forward or final hops (or both)
        info!(
            "Starting the mixnet listener... on {} (forward: {}, final hop: {}))",
            self.config.mixnet.bind_address,
            processing_config.forward_hop_processing_enabled,
            processing_config.final_hop_processing_enabled
        );

        let mixnet_client_config = nym_mixnet_client::Config::new(
            self.config.mixnet.debug.packet_forwarding_initial_backoff,
            self.config.mixnet.debug.packet_forwarding_maximum_backoff,
            self.config.mixnet.debug.initial_connection_timeout,
            self.config.mixnet.debug.maximum_connection_buffer_size,
        );
        let mixnet_client = nym_mixnet_client::Client::new(
            mixnet_client_config,
            noise_config.clone(),
            self.metrics
                .network
                .active_egress_mixnet_connections_counter(),
        );
        let active_connections = mixnet_client.active_connections();

        let mut packet_forwarder = PacketForwarder::new(
            mixnet_client,
            routing_filter,
            self.metrics.clone(),
            shutdown.clone_with_suffix("mix-packet-forwarder"),
        );
        let mix_packet_sender = packet_forwarder.sender();
        tokio::spawn(async move { packet_forwarder.run().await });

        let final_hop_data = SharedFinalHopData::new(
            active_clients_store.clone(),
            self.entry_gateway.client_storage.clone(),
        );

        let shared = mixnet::SharedData::new(
            processing_config,
            self.active_sphinx_keys()?,
            replay_protection_bloomfilter,
            mix_packet_sender.clone(),
            final_hop_data,
            noise_config,
            self.metrics.clone(),
            shutdown,
        );

        mixnet::Listener::new(self.config.mixnet.bind_address, shared).start();
        Ok((mix_packet_sender, active_connections))
    }

    pub(crate) async fn run_minimal_mixnet_processing(self) -> Result<(), NymNodeError> {
        let noise_config = nym_noise::config::NoiseConfig::new(
            self.x25519_noise_keys.clone(),
            NoiseNetworkView::new_empty(),
        )
        .with_unsafe_disabled(true);

        self.start_mixnet_listener(
            &ActiveClientsStore::new(),
            ReplayProtectionBloomfilters::new_disabled(),
            OpenFilter,
            noise_config,
            self.shutdown_manager.clone_token("mixnet-traffic"),
        )
        .await?;

        self.shutdown_manager.close();
        self.shutdown_manager.wait_for_shutdown_signal().await;

        Ok(())
    }

    pub(crate) async fn run(mut self) -> Result<(), NymNodeError> {
        info!("starting Nym Node {} with the following modes: mixnode: {}, entry: {}, exit: {}, wireguard: {}",
            self.ed25519_identity_key(),
            self.config.modes.mixnode,
            self.config.modes.entry,
            self.config.modes.exit,
            self.config.wireguard.enabled
        );
        debug!("config: {:#?}", self.config);

        let http_server = self.build_http_server().await?;
        let bind_address = self.config.http.bind_address;
        let server_shutdown = self.shutdown_manager.clone_token("http-server");

        self.shutdown_manager.spawn(async move {
            {
                info!("starting NymNodeHTTPServer on {bind_address}");
                http_server
                    .with_graceful_shutdown(async move { server_shutdown.cancelled().await })
                    .await
            }
        });

        let nym_apis_client = self.setup_nym_apis_client()?;

        self.try_refresh_remote_nym_api_cache(&nym_apis_client)
            .await?;
        self.start_verloc_measurements();

        let network_refresher = self.build_network_refresher().await?;
        let active_clients_store = ActiveClientsStore::new();

        let noise_config = nym_noise::config::NoiseConfig::new(
            self.x25519_noise_keys.clone(),
            network_refresher.noise_view(),
        )
        .with_unsafe_disabled(self.config.mixnet.debug.unsafe_disable_noise);

        let (mix_packet_sender, active_egress_mixnet_connections) = self
            .start_mixnet_listener(
                &active_clients_store,
                bloomfilters_manager.bloomfilters(),
                network_refresher.routing_filter(),
                noise_config,
                self.shutdown_manager.clone_token("mixnet-traffic"),
            )
            .await?;

        let metrics_sender = self.setup_metrics_backend(
            active_clients_store.clone(),
            active_egress_mixnet_connections,
            self.shutdown_manager.clone_token("metrics"),
        );

        self.start_gateway_tasks(
            network_refresher.cached_network(),
            metrics_sender,
            active_clients_store,
            mix_packet_sender,
            self.shutdown_manager.subscribe_legacy("gateway-tasks"),
        )
        .await?;

        self.setup_key_rotation(nym_apis_client, bloomfilters_manager)
            .await?;

        network_refresher.start();

        self.shutdown_manager.close();
        self.shutdown_manager.wait_for_shutdown_signal().await;

        Ok(())
    }
}

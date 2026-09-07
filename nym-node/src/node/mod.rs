// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use self::helpers::load_x25519_wireguard_keypair;
use crate::config::helpers::gateway_tasks_config;
use crate::config::{Config, DEFAULT_MIXNET_PORT, GatewayTasksConfig, NodeModes, Wireguard};
use crate::error::{EntryGatewayError, NymNodeError, ServiceProvidersError};
use crate::node::description::save_node_description;
use crate::node::helpers::{
    DisplayDetails, get_current_rotation_id, load_ed25519_identity_keypair, load_mceliece_keypair,
    load_mlkem768_keypair, load_x25519_lp_keypair, load_x25519_noise_keypair,
    store_ed25519_identity_keypair, store_keypair, store_mceliece_keypair, store_mlkem768_keypair,
    store_x25519_lp_keypair, store_x25519_noise_keypair,
};
use crate::node::http::api::api_requests;
use crate::node::http::state::AppState;
use crate::node::http::{HttpServerConfig, NymNodeHttpServer, NymNodeRouter};
use crate::node::key_rotation::active_keys::ActiveSphinxKeys;
use crate::node::key_rotation::controller::KeyRotationController;
use crate::node::key_rotation::manager::SphinxKeyManager;
use crate::node::lp::active_sessions::ActiveLpSessions;
use crate::node::lp::control::LpControlSetup;
use crate::node::lp::control::egress::dialer::LpDialer;
use crate::node::lp::data::LpDataSetup;
use crate::node::lp::directory::LpNodes;
use crate::node::metrics::aggregator::MetricsAggregator;
use crate::node::metrics::console_logger::ConsoleLogger;
use crate::node::metrics::handler::client_sessions::GatewaySessionStatsHandler;
use crate::node::metrics::handler::global_prometheus_updater::PrometheusGlobalNodeMetricsRegistryUpdater;
use crate::node::metrics::handler::legacy_packet_data::LegacyMixingStatsUpdater;
use crate::node::metrics::handler::mixnet_data_cleaner::MixnetMetricsCleaner;
use crate::node::metrics::handler::pending_egress_packets_updater::PendingEgressPacketsUpdater;
use crate::node::metrics::handler::tokio_runtime_updater::TokioRuntimeMetricsUpdater;
use crate::node::mixnet::SharedFinalHopData;
use crate::node::mixnet::packet_forwarding::PacketForwarder;
use crate::node::mixnet::shared::ProcessingConfig;
use crate::node::nym_apis_client::NymApisClient;
use crate::node::nyx_client::NyxClient;
use crate::node::nyxd_watcher::network_monitor_agents::NetworkMonitorAgentsModule;
use crate::node::replay_protection::background_task::ReplayProtectionDiskFlush;
use crate::node::replay_protection::bloomfilter::ReplayProtectionBloomfilters;
use crate::node::replay_protection::manager::ReplayProtectionBloomfiltersManager;
use crate::node::routing_filter::network_filter::{NetworkRoutingFilter, RoutableNetworkMonitors};
use crate::node::routing_filter::{OpenFilter, RoutingFilter};
use crate::node::shared_network::refresher::{NetworkRefresher, NetworkRefresherConfig};
use crate::node::shared_network::topology_provider::{CachedTopologyProvider, LocalGatewayNode};
use crate::node::shared_network::{CachedFullTopology, CachedNetwork};
use getrandom04::SysRng;
use nym_bin_common::bin_info;
use nym_config::defaults::NymNetworkDetails;
use nym_credential_verification::UpgradeModeState;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_gateway::node::ClientRegistry;
use nym_gateway::node::wireguard::PeerRegistrator;
use nym_gateway::node::{GatewayTasksBuilder, UpgradeModeCheckRequestSender};
use nym_kkt::key_utils::{
    generate_keypair_mceliece, generate_keypair_mlkem, generate_lp_keypair_x25519,
};
use nym_kkt::keys::{DHKeyPair, KEMKeys};
use nym_lp::Ciphersuite;
use nym_lp::peer::LpLocalPeer;
use nym_mixnet_client::client::ActiveConnections;
use nym_mixnet_client::forwarder::MixForwardingSender;
use nym_node_metrics::NymNodeMetrics;
use nym_node_metrics::events::MetricEventsSender;
use nym_noise::config::{NetworkMonitorAgentNode, NoiseConfig, NoiseNetworkView};
use nym_noise_keys::VersionedNoiseKeyV1;
use nym_task::{ShutdownManager, ShutdownToken, ShutdownTracker};
use nym_validator_client::UserAgent;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::nyxd::contract_traits::PagedNetworkMonitorsQueryClient;
use nym_validator_client::nyxd::nym_network_monitors_contract_common::AuthorisedNetworkMonitor;
use nym_verloc::measurements::SharedVerlocStats;
use nym_verloc::{self, measurements::VerlocMeasurer};
use nym_wireguard::{WireguardGatewayData, peer_controller::PeerControlRequest};
use nyxd_scraper_shared::watcher::{NyxdWatcher, WatcherConfig};
use rand::rngs::OsRng;
use rand010::SeedableRng;
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, SocketAddr};
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::WaitForCancellationFutureOwned;
use tracing::{debug, error, info, trace, warn};
use zeroize::Zeroizing;

use crate::node::directory_publisher::{
    DirectoryPublisher, DirectoryPublisherConfig, DirectoryPublisherEventsSender,
};
use crate::node::node_details::{NodeDescription, NodeDetails, ServiceProvidersKeys};
pub use nym_gateway::node::ActiveClientsStore;
use nym_gateway::node::EmbeddedServiceProviders;
pub use nym_gateway::node::GatewayStorage;

pub mod bonding_information;
pub mod description;
pub(crate) mod directory_publisher;
pub mod helpers;
pub(crate) mod http;
pub(crate) mod key_rotation;
pub mod lp;
pub(crate) mod metrics;
pub(crate) mod mixnet;
pub(crate) mod node_details;
mod nym_apis_client;
mod nyx_client;
mod nyxd_watcher;
pub(crate) mod replay_protection;
mod routing_filter;
mod shared_network;

pub struct GatewayTasksData {
    client_storage: GatewayStorage,
    stats_storage: nym_gateway::node::PersistentStatsStorage,
}

impl GatewayTasksData {
    async fn new(config: &GatewayTasksConfig) -> Result<GatewayTasksData, EntryGatewayError> {
        let client_storage = GatewayStorage::init(
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
            client_storage,
            stats_storage,
        })
    }
}

pub struct WireguardData {
    inner: WireguardGatewayData,
    peer_rx: mpsc::Receiver<PeerControlRequest>,
    use_userspace: bool,
}

impl WireguardData {
    pub(crate) fn new(config: &Wireguard) -> Result<Self, NymNodeError> {
        let (inner, peer_rx) = WireguardGatewayData::new(
            config.clone().into(),
            Arc::new(load_x25519_wireguard_keypair(
                &config.storage_paths.x25519_wireguard_storage_paths(),
            )?),
        );
        Ok(WireguardData {
            inner,
            peer_rx,
            use_userspace: config.use_userspace,
        })
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
            use_userspace: value.use_userspace,
        }
    }
}

pub struct NymNode {
    config: Config,
    shutdown_manager: ShutdownManager,

    public_details: NodeDetails,

    network: NymNetworkDetails,

    metrics: NymNodeMetrics,

    verloc_stats: SharedVerlocStats,

    entry_gateway: GatewayTasksData,

    upgrade_mode_state: UpgradeModeState,

    nyx_client: NyxClient,

    wireguard: Option<WireguardData>,

    ed25519_identity_keys: Arc<ed25519::KeyPair>,
    sphinx_key_manager: Option<SphinxKeyManager>,

    x25519_noise_keys: Arc<x25519::KeyPair>,

    psq_kem_keys: KEMKeys,
    x25519_lp_keys: Arc<DHKeyPair>,
}

impl NymNode {
    pub(crate) async fn initialise(
        config: &Config,
        custom_mnemonic: Option<Zeroizing<bip39::Mnemonic>>,
    ) -> Result<(), NymNodeError> {
        info!("initialising nym-node with id: {}", config.id);
        let mut rng = OsRng;
        let mut rng010 = rand010::rngs::StdRng::try_from_rng(&mut SysRng)?;

        // global initialisation
        info!("generating new node keys (this might take a while)");
        let ed25519_identity_keys = ed25519::KeyPair::new(&mut rng);
        let x25519_noise_keys = x25519::KeyPair::new(&mut rng);

        let x25519_lp_keys = generate_lp_keypair_x25519(&mut rng010);
        let mlkem = generate_keypair_mlkem(&mut rng010);
        let mceliece = generate_keypair_mceliece(&mut rng010);

        let current_rotation_id =
            get_current_rotation_id(&config.mixnet.nym_api_urls, &config.nyx.nyxd_urls).await?;
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

        trace!("attempting to store x25519 lp keypair");
        store_x25519_lp_keypair(
            &x25519_lp_keys,
            &config.storage_paths.keys.x25519_lp_key_paths(),
        )?;

        trace!("attempting to store mlkem768 keypair");
        store_mlkem768_keypair(&mlkem, &config.storage_paths.keys.mlkem768_key_paths())?;

        trace!("attempting to store mceliece keypair");
        store_mceliece_keypair(&mceliece, &config.storage_paths.keys.mceliece_key_paths())?;

        trace!("creating description file");
        save_node_description(
            &config.storage_paths.description,
            &NodeDescription::default(),
        )?;
        let mnemonic = match custom_mnemonic {
            None => {
                trace!("generating new mnemonic");
                // SAFETY: 24 is a valid word count
                #[allow(clippy::unwrap_used)]
                Arc::new(Zeroizing::new(bip39::Mnemonic::generate(24).unwrap()))
            }
            Some(custom_mnemonic) => Arc::new(custom_mnemonic),
        };

        trace!("attempting to store the mnemonic");
        config.storage_paths.save_mnemonic_to_file(&mnemonic)?;

        // service providers initialisation
        ServiceProvidersKeys::initialise(
            &config.service_providers,
            *ed25519_identity_keys.public_key(),
        )
        .await?;

        // wireguard initialisation
        WireguardData::initialise(&config.wireguard)?;

        config.save()
    }

    pub async fn build_lp_control_tasks(
        &self,
        peer_registrator: Option<PeerRegistrator>,
        network_nodes: LpNodes,
        sessions: ActiveLpSessions,
        clients: ClientRegistry,
    ) -> Result<LpControlSetup, NymNodeError> {
        let lp_peer = LpLocalPeer::new(Ciphersuite::default(), self.x25519_lp_keys.clone())
            .with_kem_keys(self.psq_kem_keys.clone());

        LpControlSetup::new(
            lp_peer,
            self.config.lp,
            self.metrics.clone(),
            peer_registrator,
            network_nodes,
            sessions,
            clients,
            self.shutdown_manager.shutdown_tracker().clone(),
        )
        .await
    }

    #[expect(clippy::too_many_arguments)]
    pub(crate) fn build_lp_data_tasks(
        &self,
        cached_network: CachedFullTopology,
        replay_protection_bloomfilter: ReplayProtectionBloomfilters,
        routing_filter: NetworkRoutingFilter,
        sessions: ActiveLpSessions,
        clients: ClientRegistry,
        service_providers: EmbeddedServiceProviders,
        dialer: LpDialer,
    ) -> Result<LpDataSetup, NymNodeError> {
        let shared_state = lp::data::shared::SharedLpDataState::new(
            self.config(),
            self.active_sphinx_keys()?,
            replay_protection_bloomfilter,
            routing_filter,
            sessions,
            clients,
            self.metrics.clone(),
            self.shutdown_token(),
        );

        // gateway-only LP data state
        let gateway_state = self.config().modes.expects_client_traffic().then(|| {
            lp::data::shared::SharedGatewayLpDataState::new(cached_network, service_providers)
        });

        LpDataSetup::new(
            shared_state,
            gateway_state,
            dialer,
            self.shutdown_manager.shutdown_tracker().clone(),
        )
    }

    pub(crate) async fn new(
        config: Config,
        accepted_operator_terms_and_conditions: bool,
    ) -> Result<Self, NymNodeError> {
        let wireguard_data = WireguardData::new(&config.wireguard)?;
        let current_rotation_id =
            get_current_rotation_id(&config.mixnet.nym_api_urls, &config.nyx.nyxd_urls).await?;

        let ed25519_identity_keys = load_ed25519_identity_keypair(
            &config.storage_paths.keys.ed25519_identity_storage_paths(),
        )?;
        let entry_gateway = GatewayTasksData::new(&config.gateway_tasks).await?;
        let x25519_lp_keys =
            load_x25519_lp_keypair(&config.storage_paths.keys.x25519_lp_key_paths())?;
        let mlkem = load_mlkem768_keypair(&config.storage_paths.keys.mlkem768_key_paths())?;
        let mceliece = load_mceliece_keypair(&config.storage_paths.keys.mceliece_key_paths())?;
        let psq_kem_keys = KEMKeys::new(mceliece, mlkem);
        let mnemonic = config.storage_paths.load_mnemonic_from_file()?;

        let network = NymNetworkDetails::new_from_env();
        let nyx_client = NyxClient::new(&config.nyx, &network, &mnemonic)?;
        let cosmos_address = nyx_client.address().await;

        let node_details = NodeDetails::construct(
            &config,
            accepted_operator_terms_and_conditions,
            psq_kem_keys.encapsulation_keys(),
            x25519_lp_keys.pk,
            cosmos_address,
        )?;

        Ok(NymNode {
            ed25519_identity_keys: Arc::new(ed25519_identity_keys),
            sphinx_key_manager: Some(SphinxKeyManager::try_load_or_regenerate(
                current_rotation_id,
                &config.storage_paths.keys.primary_x25519_sphinx_key_file,
                &config.storage_paths.keys.secondary_x25519_sphinx_key_file,
            )?),
            x25519_noise_keys: Arc::new(load_x25519_noise_keypair(
                &config.storage_paths.keys.x25519_noise_storage_paths(),
            )?),
            psq_kem_keys,
            metrics: NymNodeMetrics::new(),
            verloc_stats: Default::default(),
            entry_gateway,
            upgrade_mode_state: UpgradeModeState::new(
                config.gateway_tasks.upgrade_mode.attester_public_key,
            ),
            nyx_client,
            wireguard: Some(wireguard_data),
            config,
            shutdown_manager: ShutdownManager::build_new_default()
                .map_err(|source| NymNodeError::ShutdownSignalFailure { source })?,
            x25519_lp_keys: Arc::new(x25519_lp_keys),
            network,
            public_details: node_details,
        })
    }

    pub(crate) fn shutdown_tracker(&self) -> &ShutdownTracker {
        self.shutdown_manager.shutdown_tracker()
    }

    pub(crate) fn shutdown_token(&self) -> ShutdownToken {
        self.shutdown_manager.clone_shutdown_token()
    }

    pub(crate) fn config(&self) -> &Config {
        &self.config
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
            description: self.public_details.description().clone(),
            ed25519_identity_key: self.ed25519_identity_key().to_base58_string(),
            x25519_primary_sphinx_key: sphinx_keys.keys.primary().deref().into(),
            x25519_secondary_sphinx_key: sphinx_keys.keys.secondary().map(|g| g.deref().into()),
            x25519_noise_key: self.x25519_noise_key().to_base58_string(),
            x25519_wireguard_key: self.x25519_wireguard_key()?.to_base58_string(),
            exit_network_requester_address: self
                .public_details
                .exit_network_requester_address()
                .to_string(),
            exit_ip_packet_router_address: self
                .public_details
                .exit_ip_packet_router_address()
                .to_string(),
            exit_authenticator_address: self
                .public_details
                .exit_authenticator_address()
                .to_string(),
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

    async fn build_network_refresher(
        &self,
        routing_filter: NetworkRoutingFilter,
        client: NymApisClient,
        noise_view: NoiseNetworkView,
        lp_nodes: LpNodes,
    ) -> Result<NetworkRefresher, NymNodeError> {
        let config = NetworkRefresherConfig::new(
            self.config.debug.topology_cache_ttl,
            self.config.debug.routing_nodes_check_interval,
            self.config
                .gateway_tasks
                .debug
                .maximum_initial_topology_waiting_time,
            self.config.gateway_tasks.debug.minimum_mix_performance,
        );
        NetworkRefresher::initialise_new(
            config,
            client,
            routing_filter,
            noise_view,
            lp_nodes,
            self.shutdown_manager.clone_shutdown_token(),
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
            active_sphinx_keys: self.active_sphinx_keys()?,
            mix_host,
            ip_addresses,
            identity_key: *self.ed25519_identity_key(),
            entry: nym_topology::EntryDetails {
                clients_ws_port,
                hostname: self.config.host.hostname.clone(),
                clients_wss_port: self.config.gateway_tasks.announce_wss_port,
            },
        })
    }

    /// Returns the WireGuard peer registrator, which the LP control plane needs for dVPN
    /// registration. It can only be built here (it needs the gateway tasks builder), so it
    /// is handed back to the caller rather than LP being set up inside this function.
    async fn start_gateway_tasks(
        &mut self,
        node_address: AccountId,
        cached_network: CachedNetwork,
        metrics_sender: MetricEventsSender,
        active_clients_store: ActiveClientsStore,
        mix_packet_sender: MixForwardingSender,
    ) -> Result<Option<PeerRegistrator>, NymNodeError> {
        let config = gateway_tasks_config(&self.config);

        let topology_provider = Box::new(CachedTopologyProvider::new(
            self.as_gateway_topology_node()?,
            cached_network,
            self.config.gateway_tasks.debug.minimum_mix_performance,
        ));

        let mut gateway_tasks_builder = GatewayTasksBuilder::new(
            config.gateway,
            self.network.clone(),
            self.ed25519_identity_keys.clone(),
            self.entry_gateway.client_storage.clone(),
            mix_packet_sender,
            metrics_sender,
            self.metrics.clone(),
            node_address,
            Self::user_agent(),
            self.upgrade_mode_state.clone(),
            self.config.lp.debug.use_mock_ecash,
            self.shutdown_tracker().clone(),
        );

        // start task for watching the changes in upgrade mode attestation
        let upgrade_check_request_sender = if let Some(upgrade_mode_watcher) =
            gateway_tasks_builder.try_build_upgrade_mode_watcher()
        {
            let req_sender = upgrade_mode_watcher.request_sender();
            upgrade_mode_watcher.start();
            req_sender
        } else {
            UpgradeModeCheckRequestSender::new_empty()
        };

        // create the common state for subtasks relying on the upgrade mode information
        // (i.e. everything that'd require ticket/bandwidth processing)
        let upgrade_mode_common_state =
            gateway_tasks_builder.build_upgrade_mode_common_state(upgrade_check_request_sender);

        // Set WireGuard data early so other builders can access it
        if self.config.wireguard.enabled {
            let Some(wg_data) = self.wireguard.take() else {
                return Err(NymNodeError::WireguardDataUnavailable);
            };
            gateway_tasks_builder.set_wireguard_data(wg_data.into());
        }

        let wg_peer_registrator = gateway_tasks_builder
            .build_peer_registrator(upgrade_mode_common_state.clone())
            .await?;

        // the wireguard branch below consumes `wg_peer_registrator`, so keep a handle to
        // return to the caller for the LP control plane
        let lp_peer_registrator = wg_peer_registrator.clone();

        if let Some(wg_peer_registrator) = wg_peer_registrator.as_ref() {
            let cleanup_task = wg_peer_registrator.cleanup_task(self.shutdown_token());
            self.shutdown_tracker().try_spawn_named(
                async move { cleanup_task.run().await },
                "StaleRegistrationRemover",
            );
        };

        // if we're running in entry mode, start the websocket
        if self.modes().entry {
            info!(
                "starting the clients websocket... on {}",
                self.config.gateway_tasks.ws_bind_address
            );
            let mut websocket = gateway_tasks_builder
                .build_websocket_listener(
                    active_clients_store.clone(),
                    upgrade_mode_common_state.clone(),
                )
                .await?;
            self.shutdown_tracker()
                .try_spawn_named(async move { websocket.run().await }, "EntryWebsocket");
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
            info!(
                "node not running in exit mode: the exit service providers (NR + IPR) will remain unavailable"
            );
        }

        // if we're running wireguard, start the authenticator
        // and the actual wireguard listener
        if self.config.wireguard.enabled {
            info!(
                "starting the wireguard tasks: authenticator service provider + wireguard peer controller"
            );

            gateway_tasks_builder.set_authenticator_opts(config.auth_opts);

            let Some(peer_registrator) = wg_peer_registrator else {
                return Err(NymNodeError::WireguardDataUnavailable);
            };

            let authenticator = gateway_tasks_builder
                .build_wireguard_authenticator(
                    peer_registrator,
                    upgrade_mode_common_state.clone(),
                    topology_provider,
                )
                .await?;
            let started_authenticator = authenticator.start_service_provider().await?;
            active_clients_store.insert_embedded(started_authenticator.handle);

            info!(
                "started authenticator at: {}",
                started_authenticator.on_start_data.address
            );

            gateway_tasks_builder
                .try_start_wireguard(upgrade_mode_common_state)
                .await
                .map_err(NymNodeError::GatewayTasksStartupFailure)?;
        } else {
            info!(
                "node not running with wireguard: authenticator service provider and wireguard will remain unavailable"
            );
        }

        // start task for removing stale and un-retrieved client messages
        let mut stale_messages_cleaner = gateway_tasks_builder.build_stale_messages_cleaner();
        let shutdown_token = self.shutdown_token();
        self.shutdown_tracker().try_spawn_named(
            async move { stale_messages_cleaner.run(shutdown_token).await },
            "StaleMessagesCleaner",
        );

        Ok(lp_peer_registrator)
    }

    pub(crate) async fn build_http_server(
        &self,
        shutdown: WaitForCancellationFutureOwned,
    ) -> Result<NymNodeHttpServer, NymNodeError> {
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

        let base_config = HttpServerConfig::new()
            .with_landing_page_assets(self.config.http.landing_page_assets_path.as_ref())
            .with_used_exit_policy(exit_policy_details)
            .with_prometheus_bearer_token(self.config.http.access_token.clone());

        // fills in mixnode, gateway, sp, etc. details based on the previously loaded public details
        let mut config = self
            .public_details
            .fill_http_app_config(&self.config, base_config);

        if let Some(path) = &self.config.gateway_tasks.storage_paths.bridge_client_params {
            config = config.with_bridge_client_params_file(path);
        }

        let app_state = AppState::new(
            self.public_details.build_http_app_static_node_information(
                self.ed25519_identity_keys.clone(),
                &self.config,
            ),
            self.active_sphinx_keys()?,
            self.metrics.clone(),
            self.verloc_stats.clone(),
            self.config
                .gateway_tasks
                .upgrade_mode
                .attestation_url
                .clone(),
            self.upgrade_mode_state.clone(),
            self.config.http.node_load_cache_ttl,
        );

        Ok(NymNodeRouter::new(config, app_state)
            .build_server(&self.config.http.bind_address, shutdown)
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
            self.shutdown_manager.clone_shutdown_token(),
        );
        verloc_measurer.set_shared_state(self.verloc_stats.clone());
        self.shutdown_manager
            .try_spawn_named(async move { verloc_measurer.run().await }, "VerlocMeasurer");
    }

    pub(crate) fn setup_metrics_backend(
        &self,
        active_clients_store: ActiveClientsStore,
        active_egress_mixnet_connections: ActiveConnections,
    ) -> MetricEventsSender {
        info!("setting up node metrics...");

        // aggregator (to listen for any metrics events)
        let mut metrics_aggregator =
            MetricsAggregator::new(self.config.metrics.debug.aggregator_update_rate);

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

        // handler sampling tokio runtime scheduling metrics (run-queue depth, busy ratio) into
        // the prometheus registry. run-queue depth is a transient gauge, so we sample at the base
        // aggregator cadence (~5s) rather than the coarse 30s global-prometheus-counters rate.
        metrics_aggregator.register_handler(
            TokioRuntimeMetricsUpdater::new(),
            self.config.metrics.debug.aggregator_update_rate,
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
            let mut console_logger = ConsoleLogger::new(
                self.config.metrics.debug.console_logging_update_interval,
                self.metrics.clone(),
            );

            self.shutdown_tracker().try_spawn_named_with_shutdown(
                async move { console_logger.run().await },
                "ConsoleLogger",
            );
        }

        let events_sender = metrics_aggregator.sender();

        // spawn the aggregator task
        let shutdown_token = self.shutdown_token();
        self.shutdown_tracker().try_spawn_named(
            async move { metrics_aggregator.run(shutdown_token).await },
            "MetricsAggregator",
        );

        events_sender
    }

    pub(crate) async fn setup_replay_detection(
        &self,
    ) -> Result<ReplayProtectionBloomfiltersManager, NymNodeError> {
        info!("setting up replay detection");

        if self.config.mixnet.replay_protection.debug.unsafe_disabled {
            warn!("replay protection is disabled");
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
            self.shutdown_manager.clone_shutdown_token(),
        )
        .await?;

        let bloomfilters_manager = replay_detection_background.bloomfilters_manager();
        self.shutdown_manager.try_spawn_named(
            async move { replay_detection_background.run().await },
            "ReplayDetection",
        );
        Ok(bloomfilters_manager)
    }

    // I'm assuming this will be needed in other places, so it's explicitly extracted
    fn setup_nym_apis_client(&self) -> Result<NymApisClient, NymNodeError> {
        NymApisClient::new(
            &self.config.mixnet.nym_api_urls,
            self.shutdown_manager.clone_shutdown_token(),
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
        directory_publisher_events_sender: Option<DirectoryPublisherEventsSender>,
    ) -> Result<(), NymNodeError> {
        let managed_keys = self.take_managed_sphinx_keys()?;
        let rotation_state = nym_apis_client.get_key_rotation_info().await?;

        let rotation_controller = KeyRotationController::new(
            &self.config,
            rotation_state.into(),
            nym_apis_client,
            replay_protection_manager,
            managed_keys,
            directory_publisher_events_sender,
            self.shutdown_manager.clone_shutdown_token(),
        );

        rotation_controller.start();
        Ok(())
    }

    pub(crate) async fn start_mixnet_listener<F>(
        &self,
        active_clients_store: &ActiveClientsStore,
        replay_protection_bloomfilter: ReplayProtectionBloomfilters,
        routing_filter: F,
        authorised_network_monitor_agents: RoutableNetworkMonitors,
        noise_config: NoiseConfig,
    ) -> Result<(MixForwardingSender, ActiveConnections), NymNodeError>
    where
        F: RoutingFilter + Send + Sync + 'static,
    {
        let processing_config = ProcessingConfig::new(&self.config);

        // pre-register the whole mixnet_packet_* histogram family so it's present on the
        // prometheus endpoint at zero from boot (not just after the first sampled packet)
        nym_mixnet_client::metrics::register_all();

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
            self.config.mixnet.debug.use_legacy_packet_encoding,
            self.config.mixnet.debug.connection_idle_timeout,
            self.config.mixnet.debug.connection_write_timeout,
        );
        let mixnet_client = nym_mixnet_client::Client::new(
            mixnet_client_config,
            noise_config.clone(),
            self.metrics
                .network
                .active_egress_mixnet_connections_counter(),
        );
        let active_connections = mixnet_client.active_connections();

        let mut packet_forwarder =
            PacketForwarder::new(mixnet_client, routing_filter, self.metrics.clone());
        let mix_packet_sender = packet_forwarder.sender();

        let shutdown_token = self.shutdown_token();
        self.shutdown_tracker().try_spawn_named(
            async move { packet_forwarder.run(shutdown_token).await },
            "PacketForwarder",
        );

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
            authorised_network_monitor_agents,
            self.shutdown_token(),
        );

        let mut mixnet_listener = mixnet::Listener::new(self.config.mixnet.bind_address, shared);

        let shutdown_token = self.shutdown_token();
        self.shutdown_tracker().try_spawn_named(
            async move { mixnet_listener.run(shutdown_token).await },
            "MixnetListener",
        );

        Ok((mix_packet_sender, active_connections))
    }

    pub(crate) async fn run_minimal_mixnet_processing(mut self) -> Result<(), NymNodeError> {
        let noise_config = NoiseConfig::new(
            self.x25519_noise_keys.clone(),
            NoiseNetworkView::new_empty(),
            self.config.mixnet.debug.initial_connection_timeout,
        )
        .with_unsafe_disabled(true);

        self.start_mixnet_listener(
            &ActiveClientsStore::new(),
            ReplayProtectionBloomfilters::new_disabled(),
            OpenFilter,
            RoutableNetworkMonitors::default(),
            noise_config,
        )
        .await?;

        self.shutdown_manager.close_tracker();
        self.shutdown_manager.run_until_shutdown().await;

        Ok(())
    }

    async fn known_network_monitors(&self) -> Result<Vec<AuthorisedNetworkMonitor>, NymNodeError> {
        info!("obtaining the list of known network monitors");

        Ok(self
            .nyx_client
            .read()
            .await
            .get_all_network_monitor_agents()
            .await?)
    }

    async fn setup_nyx_chain_watcher(
        &self,
        network_monitors_handle: RoutableNetworkMonitors,
        noise_network_view: NoiseNetworkView,
    ) -> Result<(), NymNodeError> {
        info!("setting up nyx chain watcher");

        // START: module creation
        let Some(Ok(contract_address)) = self
            .network
            .contracts
            .network_monitors_contract_address
            .as_ref()
            .map(|addr| addr.parse())
        else {
            // **THEORETICALLY** this should be impossible, for we have already created a nyxd client and
            // queried this very contract before
            return Err(NymNodeError::MissingNetworkMonitorsContractAddress);
        };
        let nm_agents = NetworkMonitorAgentsModule::new(
            contract_address,
            network_monitors_handle,
            noise_network_view,
        );

        // END: module creation
        let cancellation = self.shutdown_manager.clone_shutdown_token();

        let config = WatcherConfig {
            websocket_url: self.config.nyx.nyxd_websocket_url.clone(),
            rpc_url: self.config.nyx.nyxd_urls[0].clone(),
        };
        let watcher = NyxdWatcher::builder(config)
            .with_msg_module(nm_agents)
            .with_custom_shutdown(cancellation.to_cancellation_token());

        watcher.build_and_start().await?;
        Ok(())
    }

    async fn setup_directory_publishing(
        &self,
    ) -> Result<Option<DirectoryPublisherEventsSender>, NymNodeError> {
        info!("setting up directory publishing");

        if !self.config.directory.enabled {
            warn!("this node will not submit any directory information");
            return Ok(None);
        }

        let config = DirectoryPublisherConfig::new(self.config.directory);
        let mut directory_publisher = DirectoryPublisher::new(
            self.nyx_client.clone(),
            config,
            self.ed25519_identity_keys.clone(),
            self.public_details.clone(),
            self.active_sphinx_keys()?,
            self.shutdown_manager.clone_shutdown_token(),
        )
        .await?;

        let events_sender = directory_publisher.events_sender();
        self.shutdown_tracker().try_spawn_named(
            async move { directory_publisher.run().await },
            "DirectoryPublisher",
        );
        Ok(Some(events_sender))
    }

    async fn start_nym_node_tasks(mut self) -> Result<ShutdownManager, NymNodeError> {
        info!(
            "starting Nym Node {} with the following modes: mixnode: {}, entry: {}, exit: {}, wireguard: {}",
            self.ed25519_identity_key(),
            self.config.modes.mixnode,
            self.config.modes.entry,
            self.config.modes.exit,
            self.config.wireguard.enabled
        );
        debug!("config: {:#?}", self.config);

        // ##### START HTTP SERVER #####
        let bind_address = self.config.http.bind_address;
        let shutdown = self
            .shutdown_manager
            .clone_shutdown_token()
            .cancelled_owned();
        let http_server = self.build_http_server(shutdown).await?;

        self.shutdown_manager.try_spawn_named(
            async move {
                info!("starting NymNodeHTTPServer on {bind_address}");
                http_server.await
            },
            "HttpApi",
        );
        // ##### END HTTP SERVER #####

        // shared client for querying nym-apis
        let nym_apis_client = self.setup_nym_apis_client()?;

        // announce current sphinx key to all nym apis
        self.try_refresh_remote_nym_api_cache(&nym_apis_client)
            .await?;

        // start verloc
        self.start_verloc_measurements();

        // obtain the initial list of known network monitors
        let known_network_monitors = self.known_network_monitors().await?;

        let mut known_network_monitor_ips = HashSet::new();
        let mut known_network_monitor_nodes: HashMap<IpAddr, Vec<NetworkMonitorAgentNode>> =
            HashMap::new();
        for agent in known_network_monitors {
            let Ok(x25519_pubkey) = x25519::PublicKey::from_base58_string(&agent.bs58_x25519_noise)
            else {
                error!(
                    "network monitor agent {} has announced an invalid noise key - ignoring",
                    agent.mixnet_address
                );
                continue;
            };

            let ip = agent.mixnet_address.ip();
            known_network_monitor_ips.insert(ip);

            let entry = known_network_monitor_nodes.entry(ip).or_default();
            entry.push(NetworkMonitorAgentNode {
                port: agent.mixnet_address.port(),
                key: VersionedNoiseKeyV1 {
                    supported_version: agent.noise_version.into(),
                    x25519_pubkey,
                },
            })
        }

        // build routing filter
        let routing_filter = NetworkRoutingFilter::new_empty(self.config.debug.testnet)
            .with_known_network_monitors(known_network_monitor_ips);
        let network_monitors_ref = routing_filter.known_network_monitors_handle();

        let noise_view = NoiseNetworkView::new_with_agents(known_network_monitor_nodes);
        let lp_nodes = LpNodes::new_empty(); // @JS Pipe NM agents here like for noise 

        // retrieve the initial view of the network and update the known set of nym nodes in the routing filter
        let network_refresher = self
            .build_network_refresher(
                routing_filter.clone(),
                nym_apis_client.clone(),
                noise_view.clone(),
                lp_nodes.clone(),
            )
            .await?;

        // setup nyx chain watcher (currently only used for updating the network monitors view)
        self.setup_nyx_chain_watcher(network_monitors_ref, noise_view.clone())
            .await?;

        let active_clients_store = ActiveClientsStore::new();

        // start building a replay detection manager (bloomfilters, etc.)
        let bloomfilters_manager = self.setup_replay_detection().await?;

        let noise_config = NoiseConfig::new(
            self.x25519_noise_keys.clone(),
            noise_view,
            self.config.mixnet.debug.initial_connection_timeout,
        )
        .with_unsafe_disabled(self.config.mixnet.debug.unsafe_disable_noise);

        // start the listener for the mixnet packet(s)
        let authorised_network_monitor_agents = routing_filter.known_network_monitors_handle();
        let (mix_packet_sender, active_egress_mixnet_connections) = self
            .start_mixnet_listener(
                &active_clients_store,
                bloomfilters_manager.bloomfilters(),
                routing_filter,
                authorised_network_monitor_agents,
                noise_config,
            )
            .await?;

        let metrics_sender = self.setup_metrics_backend(
            active_clients_store.clone(),
            active_egress_mixnet_connections,
        );

        let directory_publisher_events_sender = self.setup_directory_publishing().await?;

        let node_address = self.public_details.cosmos_address().clone();

        let lp_peer_registrator = self
            .start_gateway_tasks(
                node_address,
                network_refresher.cached_network(),
                metrics_sender,
                active_clients_store.clone(),
                mix_packet_sender,
            )
            .await?;

        // LP: control plane (TCP) and data plane (UDP)
        info!(
            "starting the LP listener on {} (data handler on: {})",
            self.config.lp.control_bind_address, self.config.lp.data_bind_address,
        );

        // sessions are established by the control plane and consumed by the data plane; the
        // registry is written by the data plane and swept by the control plane's cleanup task
        let sessions = ActiveLpSessions::new();
        let clients = ClientRegistry::default();

        let lp_control_tasks = self
            .build_lp_control_tasks(
                lp_peer_registrator,
                network_refresher.lp_nodes(),
                sessions.clone(),
                clients.clone(),
            )
            .await?;

        // taken before the setup is consumed: the data plane asks for sessions through this
        let dialer = lp_control_tasks.dialer();
        lp_control_tasks.start_tasks();

        let lp_data_tasks = self.build_lp_data_tasks(
            network_refresher.full_topology(),
            bloomfilters_manager.bloomfilters(),
            network_refresher.routing_filter(),
            sessions,
            clients,
            // taken after the providers have started, so the snapshot is complete
            active_clients_store.embedded_service_providers(),
            dialer,
        )?;
        lp_data_tasks.start_tasks();

        network_refresher.start();
        // start watching for key rotation and update the keys accordingly
        self.setup_key_rotation(
            nym_apis_client,
            bloomfilters_manager,
            directory_publisher_events_sender,
        )
        .await?;

        self.shutdown_manager.close_tracker();

        Ok(self.shutdown_manager)
    }

    pub async fn run(mut self) -> Result<(), NymNodeError> {
        let mut shutdown_signals = self.shutdown_manager.detach_shutdown_signals();

        // listen for shutdown signal in case we received it when attempting to spawn all the tasks
        tokio::select! {
            _ = shutdown_signals.wait_for_signal() => {
                info!("received shutdown signal during setup - exiting");
                // ideally we'd also do some cleanup here, but currently there's no easy way to access the handles
                return Ok(())
            }
            startup_result = self.start_nym_node_tasks() => {
                let mut shutdown_manager = startup_result?;
                shutdown_manager.replace_shutdown_signals(shutdown_signals);
                shutdown_manager.run_until_shutdown().await;
            }
        }

        Ok(())
    }
}

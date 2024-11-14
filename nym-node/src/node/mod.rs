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
    load_ed25519_identity_keypair, load_key, load_x25519_noise_keypair, load_x25519_sphinx_keypair,
    store_ed25519_identity_keypair, store_key, store_keypair, store_x25519_noise_keypair,
    store_x25519_sphinx_keypair, DisplayDetails,
};
use crate::node::http::api::api_requests;
use crate::node::http::helpers::sign_host_details;
use crate::node::http::helpers::system_info::get_system_info;
use crate::node::http::state::AppState;
use crate::node::http::{HttpServerConfig, NymNodeHTTPServer, NymNodeRouter};
use crate::node::metrics::aggregator::MetricsAggregator;
use crate::node::metrics::console_logger::ConsoleLogger;
use crate::node::metrics::handler::client_sessions::GatewaySessionStatsHandler;
use crate::node::metrics::handler::legacy_packet_data::LegacyMixingStatsUpdater;
use crate::node::metrics::handler::mixnet_data_cleaner::MixnetMetricsCleaner;
use crate::node::mixnet::packet_forwarding::PacketForwarder;
use crate::node::mixnet::SharedFinalHopData;
use crate::node::shared_topology::NymNodeTopologyProvider;
use nym_bin_common::{bin_info, bin_info_owned};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_gateway::node::{ActiveClientsStore, GatewayTasksBuilder};
use nym_mixnet_client::forwarder::MixForwardingSender;
use nym_network_requester::{
    set_active_gateway, setup_fs_gateways_storage, store_gateway_details, CustomGatewayDetails,
    GatewayDetails, GatewayRegistration,
};
use nym_node_metrics::events::MetricEventsSender;
use nym_node_metrics::NymNodeMetrics;
use nym_node_requests::api::v1::node::models::{AnnouncePorts, NodeDescription};
use nym_sphinx_acknowledgements::AckKey;
use nym_sphinx_addressing::Recipient;
use nym_task::{TaskClient, TaskManager};
use nym_topology::NetworkAddress;
use nym_validator_client::client::NymApiClientExt;
use nym_validator_client::models::NodeRefreshBody;
use nym_validator_client::NymApiClient;
use nym_verloc::measurements::SharedVerlocStats;
use nym_verloc::{self, measurements::VerlocMeasurer};
use nym_wireguard::{peer_controller::PeerControlRequest, WireguardGatewayData};
use rand::rngs::OsRng;
use rand::{CryptoRng, RngCore};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::{debug, info, trace, warn};
use zeroize::Zeroizing;

pub mod bonding_information;
pub mod description;
pub mod helpers;
pub(crate) mod http;
pub(crate) mod metrics;
pub(crate) mod mixnet;
mod shared_topology;

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
            ed25519_paths,
            format!("{typ}-ed25519-identity"),
        )?;
        store_keypair(&x25519_keys, x25519_paths, format!("{typ}-x25519-dh"))?;
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
                config.storage_paths.x25519_wireguard_storage_paths(),
            )?),
        );
        Ok(WireguardData { inner, peer_rx })
    }

    pub(crate) fn initialise(config: &Wireguard) -> Result<(), ServiceProvidersError> {
        let mut rng = OsRng;
        let x25519_keys = x25519::KeyPair::new(&mut rng);

        store_keypair(
            &x25519_keys,
            config.storage_paths.x25519_wireguard_storage_paths(),
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

    description: NodeDescription,

    metrics: NymNodeMetrics,

    verloc_stats: SharedVerlocStats,

    entry_gateway: GatewayTasksData,

    #[allow(dead_code)]
    service_providers: ServiceProvidersData,

    wireguard: Option<WireguardData>,

    ed25519_identity_keys: Arc<ed25519::KeyPair>,
    x25519_sphinx_keys: Arc<x25519::KeyPair>,

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
        let x25519_sphinx_keys = x25519::KeyPair::new(&mut rng);
        let x25519_noise_keys = x25519::KeyPair::new(&mut rng);

        trace!("attempting to store ed25519 identity keypair");
        store_ed25519_identity_keypair(
            &ed25519_identity_keys,
            config.storage_paths.keys.ed25519_identity_storage_paths(),
        )?;

        trace!("attempting to store x25519 sphinx keypair");
        store_x25519_sphinx_keypair(
            &x25519_sphinx_keys,
            config.storage_paths.keys.x25519_sphinx_storage_paths(),
        )?;

        trace!("attempting to store x25519 noise keypair");
        store_x25519_noise_keypair(
            &x25519_noise_keys,
            config.storage_paths.keys.x25519_noise_storage_paths(),
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

        Ok(NymNode {
            ed25519_identity_keys: Arc::new(load_ed25519_identity_keypair(
                config.storage_paths.keys.ed25519_identity_storage_paths(),
            )?),
            x25519_sphinx_keys: Arc::new(load_x25519_sphinx_keypair(
                config.storage_paths.keys.x25519_sphinx_storage_paths(),
            )?),
            x25519_noise_keys: Arc::new(load_x25519_noise_keypair(
                config.storage_paths.keys.x25519_noise_storage_paths(),
            )?),
            description: load_node_description(&config.storage_paths.description)?,
            metrics: NymNodeMetrics::new(),
            verloc_stats: Default::default(),
            entry_gateway: GatewayTasksData::new(&config.gateway_tasks).await?,
            service_providers: ServiceProvidersData::new(&config.service_providers)?,
            wireguard: Some(wireguard_data),
            config,
            accepted_operator_terms_and_conditions: false,
        })
    }

    pub(crate) fn config(&self) -> &Config {
        &self.config
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
        Ok(DisplayDetails {
            current_modes: self.config.modes,
            description: self.description.clone(),
            ed25519_identity_key: self.ed25519_identity_key().to_base58_string(),
            x25519_sphinx_key: self.x25519_sphinx_key().to_base58_string(),
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

    pub(crate) fn x25519_sphinx_key(&self) -> &x25519::PublicKey {
        self.x25519_sphinx_keys.public_key()
    }

    pub(crate) fn x25519_noise_key(&self) -> &x25519::PublicKey {
        self.x25519_noise_keys.public_key()
    }

    // the reason it's here as opposed to in the gateway directly,
    // is that other nym-node tasks will also eventually need it
    // (such as the ones for obtaining noise keys of other nodes)
    fn build_topology_provider(&self) -> Result<NymNodeTopologyProvider, NymNodeError> {
        Ok(NymNodeTopologyProvider::new(
            self.as_gateway_topology_node()?,
            self.config.debug.topology_cache_ttl,
            bin_info!().into(),
            self.config.mixnet.nym_api_urls.clone(),
        ))
    }

    fn as_gateway_topology_node(&self) -> Result<nym_topology::gateway::LegacyNode, NymNodeError> {
        let Some(ip) = self.config.host.public_ips.first() else {
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
            .unwrap_or(self.config.gateway_tasks.bind_address.port());

        Ok(nym_topology::gateway::LegacyNode {
            node_id: u32::MAX,
            mix_host,
            host: NetworkAddress::IpAddr(*ip),
            clients_ws_port,
            clients_wss_port: self.config.gateway_tasks.announce_wss_port,
            sphinx_key: *self.x25519_sphinx_key(),
            identity_key: *self.ed25519_identity_key(),
            version: env!("CARGO_PKG_VERSION").into(),
        })
    }

    async fn start_gateway_tasks(
        &mut self,
        metrics_sender: MetricEventsSender,
        active_clients_store: ActiveClientsStore,
        mix_packet_sender: MixForwardingSender,
        task_client: TaskClient,
    ) -> Result<(), NymNodeError> {
        let config = gateway_tasks_config(&self.config);
        let topology_provider = Box::new(self.build_topology_provider()?);

        let mut gateway_tasks_builder = GatewayTasksBuilder::new(
            config.gateway,
            self.ed25519_identity_keys.clone(),
            self.entry_gateway.client_storage.clone(),
            mix_packet_sender,
            metrics_sender,
            self.entry_gateway.mnemonic.clone(),
            task_client,
        );

        // if we're running in entry mode, start the websocket
        if self.modes().entry {
            info!("starting the clients websocket...");
            let websocket = gateway_tasks_builder
                .build_websocket_listener(active_clients_store.clone())
                .await?;
            websocket.start();
        }

        // if we're running in exit mode, start the IPR and NR
        if self.modes().exit {
            gateway_tasks_builder.set_network_requester_opts(config.nr_opts);
            gateway_tasks_builder.set_ip_packet_router_opts(config.ipr_opts);

            info!("starting the exit service providers (nr/ipr)...");
            let exit_sps = gateway_tasks_builder.build_exit_service_providers(
                topology_provider.clone(),
                topology_provider.clone(),
            )?;

            // note, this has all the joinhandles for when we want to use joinset
            let (started_nr, started_ipr) = exit_sps.start_service_providers().await?;
            active_clients_store.insert_embedded(started_nr.handle);
            active_clients_store.insert_embedded(started_ipr.handle);
        }

        // if we're running wireguard, start the authenticator
        // and the actual wireguard listener
        if self.config.wireguard.enabled {
            gateway_tasks_builder.set_authenticator_opts(config.auth_opts);

            // that's incredibly nasty, but unfortunately to change it, would require some refactoring...
            let Some(wg_data) = self.wireguard.take() else {
                return Err(NymNodeError::WireguardDataUnavailable);
            };

            gateway_tasks_builder.set_wireguard_data(wg_data.into());

            info!("starting wireguard + authenticator...");
            let authenticator = gateway_tasks_builder
                .build_wireguard_authenticator(topology_provider)
                .await?;
            let started_authenticator = authenticator.start_service_provider().await?;
            active_clients_store.insert_embedded(started_authenticator.handle);

            gateway_tasks_builder
                .try_start_wireguard()
                .await
                .map_err(NymNodeError::GatewayTasksStartupFailure)?;
        }

        Ok(())
    }

    pub(crate) async fn build_http_server(&self) -> Result<NymNodeHTTPServer, NymNodeError> {
        let host_details = sign_host_details(
            &self.config,
            self.x25519_sphinx_keys.public_key(),
            self.x25519_noise_keys.public_key(),
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
                .unwrap_or(self.config.gateway_tasks.bind_address.port()),
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

        let mut config = HttpServerConfig::new(host_details)
            .with_landing_page_assets(self.config.http.landing_page_assets_path.as_ref())
            .with_mixnode_details(mixnode_details)
            .with_gateway_details(gateway_details)
            .with_network_requester_details(nr_details)
            .with_ip_packet_router_details(ipr_details)
            .with_authenticator_details(auth_details)
            .with_used_exit_policy(exit_policy_details)
            .with_description(self.description.clone())
            .with_auxiliary_details(auxiliary_details);

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

        let app_state = AppState::new(self.metrics.clone(), self.verloc_stats.clone())
            .with_metrics_key(self.config.http.access_token.clone());

        Ok(NymNodeRouter::new(config, app_state)
            .build_server(&self.config.http.bind_address)
            .await?)
    }

    async fn try_refresh_remote_nym_api_cache(&self) {
        info!("attempting to request described cache request from nym-api...");
        if self.config.mixnet.nym_api_urls.is_empty() {
            warn!("no nym-api urls available");
            return;
        }

        for nym_api in &self.config.mixnet.nym_api_urls {
            info!("trying {nym_api}...");
            let client = NymApiClient::new_with_user_agent(nym_api.clone(), bin_info_owned!());

            // make new request every time in case previous one takes longer and invalidates the signature
            let request = NodeRefreshBody::new(self.ed25519_identity_keys.private_key());
            match timeout(
                Duration::from_secs(10),
                client.nym_api.force_refresh_describe_cache(&request),
            )
            .await
            {
                Ok(Ok(_)) => {
                    info!("managed to refresh own self-described data cache")
                }
                Ok(Err(request_failure)) => {
                    warn!("failed to resolve the refresh request: {request_failure}")
                }
                Err(_timeout) => {
                    warn!("timed out while attempting to resolve the request. the cache might be stale")
                }
            };
        }
    }

    pub(crate) fn start_verloc_measurements(&self, shutdown: TaskClient) {
        info!("Starting the round-trip-time measurer...");

        let config =
            nym_verloc::measurements::ConfigBuilder::new(self.config.mixnet.nym_api_urls.clone())
                .listening_address(self.config.verloc.bind_address)
                .packets_per_node(self.config.verloc.debug.packets_per_node)
                .connection_timeout(self.config.verloc.debug.connection_timeout)
                .packet_timeout(self.config.verloc.debug.packet_timeout)
                .delay_between_packets(self.config.verloc.debug.delay_between_packets)
                .tested_nodes_batch_size(self.config.verloc.debug.tested_nodes_batch_size)
                .testing_interval(self.config.verloc.debug.testing_interval)
                .retry_timeout(self.config.verloc.debug.retry_timeout)
                .build();

        let mut verloc_measurer =
            VerlocMeasurer::new(config, self.ed25519_identity_keys.clone(), shutdown);
        verloc_measurer.set_shared_state(self.verloc_stats.clone());
        tokio::spawn(async move { verloc_measurer.run().await });
    }

    pub(crate) fn setup_metrics_backend(&self, shutdown: TaskClient) -> MetricEventsSender {
        info!("setting up node metrics...");

        // aggregator (to listen for any metrics events)
        let mut metrics_aggregator = MetricsAggregator::new(
            self.config.metrics.debug.aggregator_update_rate,
            shutdown.fork("aggregator"),
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

        // handler for periodically cleaning up stale recipient/sender darta
        metrics_aggregator.register_handler(
            MixnetMetricsCleaner::new(self.metrics.clone()),
            self.config.metrics.debug.stale_mixnet_metrics_cleaner_rate,
        );

        // note: we're still measuring things such as number of mixed packets,
        // but since they're stored as atomic integers, they are incremented directly at source
        // rather than going through event pipeline
        // should we need custom mixnet events, we can add additional handler for that. that's not a problem

        // >>>> END: register all relevant handlers

        // console logger to preserve old mixnode functionalities
        // if self.config.logging.debug.log_to_console {
        if self.config.metrics.debug.log_stats_to_console {
            ConsoleLogger::new(
                self.config.metrics.debug.console_logging_update_interval,
                self.metrics.clone(),
                shutdown.named("metrics-console-logger"),
            )
            .start();
        } else {
            let mut shutdown = shutdown;
            shutdown.disarm()
        }

        let events_sender = metrics_aggregator.sender();

        // spawn the aggregator task
        metrics_aggregator.start();

        events_sender
    }

    pub(crate) fn start_mixnet_listener(
        &self,
        active_clients_store: &ActiveClientsStore,
        shutdown: TaskClient,
    ) -> MixForwardingSender {
        // we're ALWAYS listening for mixnet packets, either for forward or final hops (or both)
        info!("Starting the mixnet listener...");

        let mixnet_client_config = nym_mixnet_client::Config::new(
            self.config.mixnet.debug.packet_forwarding_initial_backoff,
            self.config.mixnet.debug.packet_forwarding_maximum_backoff,
            self.config.mixnet.debug.initial_connection_timeout,
            self.config.mixnet.debug.maximum_connection_buffer_size,
        );
        let mixnet_client = nym_mixnet_client::Client::new(mixnet_client_config);

        let mut packet_forwarder = PacketForwarder::new(
            mixnet_client,
            self.metrics.clone(),
            shutdown.fork("mix-packet-forwarder"),
        );
        let mix_packet_sender = packet_forwarder.sender();
        tokio::spawn(async move { packet_forwarder.run().await });

        let final_hop_data = SharedFinalHopData::new(
            active_clients_store.clone(),
            self.entry_gateway.client_storage.clone(),
        );

        let shared = mixnet::SharedData::new(
            &self.config,
            self.x25519_sphinx_keys.private_key(),
            mix_packet_sender.clone(),
            final_hop_data,
            self.metrics.clone(),
            shutdown,
        );

        mixnet::Listener::new(self.config.mixnet.bind_address, shared).start();
        mix_packet_sender
    }

    pub(crate) async fn run(mut self) -> Result<(), NymNodeError> {
        info!("starting Nym Node {}", self.ed25519_identity_key());

        let mut task_manager = TaskManager::default().named("NymNode");
        let http_server = self
            .build_http_server()
            .await?
            .with_task_client(task_manager.subscribe_named("http-server"));
        let bind_address = self.config.http.bind_address;
        tokio::spawn(async move {
            {
                info!("Started NymNodeHTTPServer on {bind_address}");
                http_server.run().await
            }
        });

        self.try_refresh_remote_nym_api_cache().await;

        self.start_verloc_measurements(task_manager.subscribe_named("verloc-measurements"));

        let metrics_sender = self.setup_metrics_backend(task_manager.subscribe_named("metrics"));
        let active_clients_store = ActiveClientsStore::new();

        let mix_packet_sender = self.start_mixnet_listener(
            &active_clients_store,
            task_manager.subscribe_named("mixnet-traffic"),
        );

        self.start_gateway_tasks(
            metrics_sender,
            active_clients_store,
            mix_packet_sender,
            task_manager.subscribe_named("gateway-tasks"),
        )
        .await?;

        let _ = task_manager.catch_interrupt().await;
        Ok(())
    }
}

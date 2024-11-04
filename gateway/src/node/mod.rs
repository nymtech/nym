// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use self::helpers::load_ip_packet_router_config;
use crate::config::Config;
use crate::error::GatewayError;
use crate::helpers::{
    load_identity_keys, override_ip_packet_router_config, override_network_requester_config,
    OverrideIpPacketRouterConfig, OverrideNetworkRequesterConfig,
};
use crate::http::HttpApiBuilder;
use crate::node::client_handling::active_clients::ActiveClientsStore;
use crate::node::client_handling::embedded_clients::{LocalEmbeddedClientHandle, MessageRouter};
use crate::node::client_handling::websocket;
use crate::node::helpers::{
    initialise_main_storage, initialise_stats_storage, load_network_requester_config,
    GatewayTopologyProvider,
};
use crate::node::mixnet_handling::receiver::connection_handler::ConnectionHandler;
use futures::channel::{mpsc, oneshot};
use nym_bin_common::bin_info;
use nym_credential_verification::ecash::{
    credential_sender::CredentialHandlerConfig, EcashManager,
};
use nym_crypto::asymmetric::{encryption, identity};
use nym_mixnet_client::forwarder::{MixForwardingSender, PacketForwarder};
use nym_network_defaults::NymNetworkDetails;
use nym_network_requester::{LocalGateway, NRServiceProviderBuilder, RequestFilter};
use nym_node_http_api::state::metrics::SharedSessionStats;
use nym_statistics_common::events::{self, StatsEventSender};
use nym_task::{TaskClient, TaskHandle, TaskManager};
use nym_topology::NetworkAddress;
use nym_types::gateway::GatewayNodeDetailsResponse;
use nym_validator_client::client::NodeId;
use nym_validator_client::nyxd::{Coin, CosmWasmClient};
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient};
use rand::seq::SliceRandom;
use rand::thread_rng;
use statistics::GatewayStatisticsCollector;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::*;

pub(crate) mod client_handling;
pub(crate) mod helpers;
pub(crate) mod mixnet_handling;
pub(crate) mod statistics;

pub use nym_gateway_stats_storage::PersistentStatsStorage;
pub use nym_gateway_storage::{PersistentStorage, Storage};

// TODO: should this struct live here?
struct StartedNetworkRequester {
    /// Request filter, either an exit policy or the allow list, used by the network requester.
    used_request_filter: RequestFilter,

    /// Handle to interact with the local network requester
    handle: LocalEmbeddedClientHandle,
}

// TODO: should this struct live here?
#[allow(unused)]
struct StartedAuthenticator {
    wg_api: Arc<nym_wireguard::WgApiWrapper>,

    /// Handle to interact with the local authenticator
    handle: LocalEmbeddedClientHandle,
}

/// Wire up and create Gateway instance
pub async fn create_gateway(
    config: Config,
    nr_config_override: Option<OverrideNetworkRequesterConfig>,
    ip_config_override: Option<OverrideIpPacketRouterConfig>,
    custom_mixnet: Option<PathBuf>,
) -> Result<Gateway, GatewayError> {
    // don't attempt to read config if NR is disabled
    let network_requester_config = if config.network_requester.enabled {
        if let Some(path) = &config.storage_paths.network_requester_config {
            let cfg = load_network_requester_config(&config.gateway.id, path).await?;
            Some(override_network_requester_config(cfg, nr_config_override))
        } else {
            // if NR is enabled, the config path must be specified
            return Err(GatewayError::UnspecifiedNetworkRequesterConfig);
        }
    } else {
        None
    };

    // don't attempt to read config if NR is disabled
    let ip_packet_router_config = if config.ip_packet_router.enabled {
        if let Some(path) = &config.storage_paths.ip_packet_router_config {
            let cfg = load_ip_packet_router_config(&config.gateway.id, path).await?;
            Some(override_ip_packet_router_config(cfg, ip_config_override))
        } else {
            // if IPR is enabled, the config path must be specified
            return Err(GatewayError::UnspecifiedIpPacketRouterConfig);
        }
    } else {
        None
    };

    let storage = initialise_main_storage(&config).await?;

    let stats_storage = initialise_stats_storage(&config).await?;

    let nr_opts = network_requester_config.map(|config| LocalNetworkRequesterOpts {
        config: config.clone(),
        custom_mixnet_path: custom_mixnet.clone(),
    });

    let ip_opts = ip_packet_router_config.map(|config| LocalIpPacketRouterOpts {
        config,
        custom_mixnet_path: custom_mixnet.clone(),
    });

    Gateway::new(config, nr_opts, ip_opts, storage, stats_storage)
}

#[derive(Debug, Clone)]
pub struct LocalNetworkRequesterOpts {
    pub config: nym_network_requester::Config,

    pub custom_mixnet_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct LocalIpPacketRouterOpts {
    pub config: nym_ip_packet_router::Config,

    pub custom_mixnet_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct LocalAuthenticatorOpts {
    pub config: nym_authenticator::Config,

    pub custom_mixnet_path: Option<PathBuf>,
}

pub struct Gateway<St = PersistentStorage> {
    config: Config,

    network_requester_opts: Option<LocalNetworkRequesterOpts>,

    ip_packet_router_opts: Option<LocalIpPacketRouterOpts>,

    // Use None when wireguard feature is not enabled too
    #[allow(dead_code)]
    authenticator_opts: Option<LocalAuthenticatorOpts>,

    /// ed25519 keypair used to assert one's identity.
    identity_keypair: Arc<identity::KeyPair>,

    /// x25519 keypair used for Diffie-Hellman. Currently only used for sphinx key derivation.
    sphinx_keypair: Arc<encryption::KeyPair>,

    client_storage: St,

    stats_storage: PersistentStatsStorage,

    wireguard_data: Option<nym_wireguard::WireguardData>,

    session_stats: Option<SharedSessionStats>,

    run_http_server: bool,
    task_client: Option<TaskClient>,
}

impl<St> Gateway<St> {
    /// Construct from the given `Config` instance.
    pub fn new(
        config: Config,
        network_requester_opts: Option<LocalNetworkRequesterOpts>,
        ip_packet_router_opts: Option<LocalIpPacketRouterOpts>,
        client_storage: St,
        stats_storage: PersistentStatsStorage,
    ) -> Result<Self, GatewayError> {
        Ok(Gateway {
            client_storage,
            stats_storage,
            identity_keypair: Arc::new(load_identity_keys(&config)?),
            sphinx_keypair: Arc::new(helpers::load_sphinx_keys(&config)?),
            config,
            network_requester_opts,
            ip_packet_router_opts,
            authenticator_opts: None,
            wireguard_data: None,
            session_stats: None,
            run_http_server: true,
            task_client: None,
        })
    }
    #[allow(clippy::too_many_arguments)]
    pub fn new_loaded(
        config: Config,
        network_requester_opts: Option<LocalNetworkRequesterOpts>,
        ip_packet_router_opts: Option<LocalIpPacketRouterOpts>,
        authenticator_opts: Option<LocalAuthenticatorOpts>,
        identity_keypair: Arc<identity::KeyPair>,
        sphinx_keypair: Arc<encryption::KeyPair>,
        client_storage: St,
        stats_storage: PersistentStatsStorage,
    ) -> Self {
        Gateway {
            config,
            network_requester_opts,
            ip_packet_router_opts,
            authenticator_opts,
            identity_keypair,
            sphinx_keypair,
            client_storage,
            stats_storage,
            wireguard_data: None,
            session_stats: None,
            run_http_server: true,
            task_client: None,
        }
    }

    pub fn disable_http_server(&mut self) {
        self.run_http_server = false
    }

    pub fn set_task_client(&mut self, task_client: TaskClient) {
        self.task_client = Some(task_client)
    }

    pub fn set_session_stats(&mut self, session_stats: SharedSessionStats) {
        self.session_stats = Some(session_stats);
    }

    pub fn set_wireguard_data(&mut self, wireguard_data: nym_wireguard::WireguardData) {
        self.wireguard_data = Some(wireguard_data)
    }

    pub async fn node_details(&self) -> Result<GatewayNodeDetailsResponse, GatewayError> {
        // TODO: this is doing redundant key loads, but I guess that's fine for now
        crate::helpers::node_details(&self.config).await
    }

    fn gateway_topology_provider(&self) -> GatewayTopologyProvider {
        GatewayTopologyProvider::new(
            self.as_topology_node(),
            bin_info!().into(),
            self.config.gateway.nym_api_urls.clone(),
        )
    }

    fn as_topology_node(&self) -> nym_topology::gateway::LegacyNode {
        let ip = self
            .config
            .host
            .public_ips
            .first()
            .copied()
            .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
        let mix_host = SocketAddr::new(ip, self.config.gateway.mix_port);

        nym_topology::gateway::LegacyNode {
            // those fields are irrelevant for the purposes of routing so it's fine if they're inaccurate.
            // the only thing that matters is the identity key (and maybe version)
            node_id: NodeId::MAX,
            mix_host,
            host: NetworkAddress::IpAddr(ip),
            clients_ws_port: self.config.gateway.clients_port,
            clients_wss_port: self.config.gateway.clients_wss_port,
            sphinx_key: *self.sphinx_keypair.public_key(),

            identity_key: *self.identity_keypair.public_key(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
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
            self.client_storage.clone(),
            ack_sender,
            active_clients_store,
        );

        let listening_address = SocketAddr::new(
            self.config.gateway.listening_address,
            self.config.gateway.mix_port,
        );

        mixnet_handling::Listener::new(listening_address, shutdown).start(connection_handler);
    }

    #[cfg(target_os = "linux")]
    async fn start_authenticator(
        &mut self,
        forwarding_channel: MixForwardingSender,
        topology_provider: GatewayTopologyProvider,
        shutdown: TaskClient,
        ecash_verifier: Arc<EcashManager<St>>,
    ) -> Result<StartedAuthenticator, Box<dyn std::error::Error + Send + Sync>>
    where
        St: Storage + Clone + 'static,
    {
        let opts = self
            .authenticator_opts
            .as_ref()
            .ok_or(GatewayError::UnspecifiedAuthenticatorConfig)?;
        let (router_tx, mut router_rx) = oneshot::channel();
        let (auth_mix_sender, auth_mix_receiver) = mpsc::unbounded();
        let router_shutdown = shutdown.fork("message_router");
        let transceiver = LocalGateway::new(
            *self.identity_keypair.public_key(),
            forwarding_channel,
            router_tx,
        );
        let all_peers = self.client_storage.get_all_wireguard_peers().await?;
        let used_private_network_ips = all_peers
            .iter()
            .cloned()
            .map(|wireguard_peer| {
                defguard_wireguard_rs::host::Peer::try_from(wireguard_peer).map(|mut peer| {
                    peer.allowed_ips
                        .pop()
                        .ok_or(Box::new(GatewayError::InternalWireguardError(format!(
                            "no private IP set for peer {}",
                            peer.public_key
                        ))))
                        .map(|p| p.ip)
                })
            })
            .collect::<Result<Result<Vec<_>, _>, _>>()??;

        if let Some(wireguard_data) = self.wireguard_data.take() {
            let (on_start_tx, on_start_rx) = oneshot::channel();
            let mut authenticator_server = nym_authenticator::Authenticator::new(
                opts.config.clone(),
                wireguard_data.inner.clone(),
                used_private_network_ips,
            )
            .with_ecash_verifier(ecash_verifier)
            .with_custom_gateway_transceiver(Box::new(transceiver))
            .with_shutdown(shutdown.fork("authenticator"))
            .with_wait_for_gateway(true)
            .with_minimum_gateway_performance(0)
            .with_custom_topology_provider(Box::new(topology_provider))
            .with_on_start(on_start_tx);

            if let Some(custom_mixnet) = &opts.custom_mixnet_path {
                authenticator_server = authenticator_server.with_stored_topology(custom_mixnet)?
            }

            tokio::spawn(async move {
                if let Err(e) = authenticator_server.run_service_provider().await {
                    log::error!("Run authenticator server - {e}");
                }
            });

            let start_data = on_start_rx
                .await
                .map_err(|_| GatewayError::AuthenticatorStartupFailure)?;

            // this should be instantaneous since the data is sent on this channel before the on start is called;
            // the failure should be impossible
            let Ok(Some(packet_router)) = router_rx.try_recv() else {
                return Err(Box::new(GatewayError::AuthenticatorStartupFailure));
            };

            MessageRouter::new(auth_mix_receiver, packet_router)
                .start_with_shutdown(router_shutdown);

            let wg_api = nym_wireguard::start_wireguard(
                self.client_storage.clone(),
                all_peers,
                shutdown,
                wireguard_data,
            )
            .await?;

            Ok(StartedAuthenticator {
                wg_api,
                handle: LocalEmbeddedClientHandle::new(start_data.address, auth_mix_sender),
            })
        } else {
            Err(Box::new(GatewayError::InternalWireguardError(
                "wireguard not set".to_string(),
            )))
        }
    }

    #[cfg(not(target_os = "linux"))]
    async fn start_authenticator(
        &self,
        _forwarding_channel: MixForwardingSender,
        _topology_provider: GatewayTopologyProvider,
        _shutdown: TaskClient,
        _ecash_verifier: Arc<EcashManager<St>>,
    ) -> Result<StartedAuthenticator, Box<dyn std::error::Error + Send + Sync>> {
        todo!("Authenticator is currently only supported on Linux");
    }

    fn start_client_websocket_listener(
        &self,
        forwarding_channel: MixForwardingSender,
        active_clients_store: ActiveClientsStore,
        shutdown: TaskClient,
        ecash_verifier: Arc<EcashManager<St>>,
        stats_event_sender: StatsEventSender,
    ) where
        St: Storage + Send + Sync + Clone + 'static,
    {
        info!("Starting client [web]socket listener...");

        let listening_address = SocketAddr::new(
            self.config.gateway.listening_address,
            self.config.gateway.clients_port,
        );

        let shared_state = websocket::CommonHandlerState {
            ecash_verifier,
            storage: self.client_storage.clone(),
            local_identity: Arc::clone(&self.identity_keypair),
            only_coconut_credentials: self.config.gateway.only_coconut_credentials,
            bandwidth_cfg: (&self.config).into(),
            stats_event_sender,
        };

        websocket::Listener::new(listening_address, shared_state).start(
            forwarding_channel,
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

    fn start_stats_collector(
        &self,
        shared_session_stats: SharedSessionStats,
        shutdown: TaskClient,
    ) -> events::StatsEventSender {
        info!("Starting gateway stats collector...");

        let (mut stats_collector, stats_event_sender) =
            GatewayStatisticsCollector::new(shared_session_stats, self.stats_storage.clone());
        tokio::spawn(async move { stats_collector.run(shutdown).await });
        stats_event_sender
    }

    // TODO: rethink the logic in this function...
    async fn start_network_requester(
        &self,
        forwarding_channel: MixForwardingSender,
        topology_provider: GatewayTopologyProvider,
        shutdown: TaskClient,
    ) -> Result<StartedNetworkRequester, GatewayError> {
        info!("Starting network requester...");

        // if network requester is enabled, configuration file must be provided!
        let Some(nr_opts) = &self.network_requester_opts else {
            return Err(GatewayError::UnspecifiedNetworkRequesterConfig);
        };

        // this gateway, whenever it has anything to send to its local NR will use fake_client_tx
        let (nr_mix_sender, nr_mix_receiver) = mpsc::unbounded();
        let router_shutdown = shutdown.fork("message_router");

        let (router_tx, mut router_rx) = oneshot::channel();

        let transceiver = LocalGateway::new(
            *self.identity_keypair.public_key(),
            forwarding_channel,
            router_tx,
        );

        let (on_start_tx, on_start_rx) = oneshot::channel();
        let mut nr_builder = NRServiceProviderBuilder::new(nr_opts.config.clone())
            .with_shutdown(shutdown)
            .with_custom_gateway_transceiver(Box::new(transceiver))
            .with_wait_for_gateway(true)
            .with_minimum_gateway_performance(0)
            .with_custom_topology_provider(Box::new(topology_provider))
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
            return Err(GatewayError::NetworkRequesterStartupFailure);
        };

        MessageRouter::new(nr_mix_receiver, packet_router).start_with_shutdown(router_shutdown);
        let address = start_data.address;

        info!("the local network requester is running on {address}",);
        Ok(StartedNetworkRequester {
            used_request_filter: start_data.request_filter,
            handle: LocalEmbeddedClientHandle::new(address, nr_mix_sender),
        })
    }

    async fn start_ip_packet_router(
        &self,
        forwarding_channel: MixForwardingSender,
        topology_provider: GatewayTopologyProvider,
        shutdown: TaskClient,
    ) -> Result<LocalEmbeddedClientHandle, GatewayError> {
        info!("Starting IP packet provider...");

        // if network requester is enabled, configuration file must be provided!
        let Some(ip_opts) = &self.ip_packet_router_opts else {
            return Err(GatewayError::UnspecifiedIpPacketRouterConfig);
        };

        // this gateway, whenever it has anything to send to its local NR will use fake_client_tx
        let (ipr_mix_sender, ipr_mix_receiver) = mpsc::unbounded();
        let router_shutdown = shutdown.fork("message_router");

        let (router_tx, mut router_rx) = oneshot::channel();

        let transceiver = LocalGateway::new(
            *self.identity_keypair.public_key(),
            forwarding_channel,
            router_tx,
        );

        let (on_start_tx, on_start_rx) = oneshot::channel();
        let mut ip_packet_router =
            nym_ip_packet_router::IpPacketRouter::new(ip_opts.config.clone())
                .with_shutdown(shutdown)
                .with_custom_gateway_transceiver(Box::new(transceiver))
                .with_wait_for_gateway(true)
                .with_minimum_gateway_performance(0)
                .with_custom_topology_provider(Box::new(topology_provider))
                .with_on_start(on_start_tx);

        if let Some(custom_mixnet) = &ip_opts.custom_mixnet_path {
            ip_packet_router = ip_packet_router.with_stored_topology(custom_mixnet)?
        }

        tokio::spawn(async move {
            if let Err(err) = ip_packet_router.run_service_provider().await {
                // no need to panic as we have passed a task client to the ip packet router so
                // we're most likely already in the process of shutting down
                error!("ip packet router has failed: {err}")
            }
        });

        let start_data = on_start_rx
            .await
            .map_err(|_| GatewayError::IpPacketRouterStartupFailure)?;

        // this should be instantaneous since the data is sent on this channel before the on start is called;
        // the failure should be impossible
        let Ok(Some(packet_router)) = router_rx.try_recv() else {
            return Err(GatewayError::IpPacketRouterStartupFailure);
        };

        MessageRouter::new(ipr_mix_receiver, packet_router).start_with_shutdown(router_shutdown);
        let address = start_data.address;

        info!("the local ip packet router is running on {address}");
        Ok(LocalEmbeddedClientHandle::new(address, ipr_mix_sender))
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
        let existing_nodes = match validator_client.get_all_basic_nodes(None).await {
            Ok(nodes) => nodes,
            Err(err) => {
                error!("failed to grab initial network gateways - {err}\n Please try to startup again in few minutes");
                return Err(GatewayError::NetworkGatewaysQueryFailure { source: err });
            }
        };

        Ok(existing_nodes
            .iter()
            .any(|node| &node.ed25519_identity_pubkey == self.identity_keypair.public_key()))
    }

    pub async fn run(mut self) -> Result<(), GatewayError>
    where
        St: Storage + Clone + 'static,
    {
        info!("Starting nym gateway!");

        if self.check_if_bonded().await? {
            warn!("You seem to have bonded your gateway before starting it - that's highly unrecommended as in the future it might result in slashing");
        }

        let shutdown = self
            .task_client
            .take()
            .map(Into::<TaskHandle>::into)
            .unwrap_or_else(|| TaskHandle::Internal(TaskManager::new(10)))
            .name_if_unnamed("gateway");

        let nyxd_client = self.random_nyxd_client()?;

        if self.config.gateway.only_coconut_credentials {
            debug!("the gateway is running in coconut-only mode - making sure it has enough tokens for credential redemption");
            let mix_denom_base = nyxd_client.current_chain_details().mix_denom.base.clone();

            let account = nyxd_client.address();
            let balance = nyxd_client
                .get_balance(&account, mix_denom_base.clone())
                .await?
                .unwrap_or(Coin::new(0, mix_denom_base));

            error!("this gateway does not have enough tokens for covering transaction fees for credential redemption");

            // see if we have at least 1nym (i.e. 1'000'000unym)
            if balance.amount < 1_000_000 {
                return Err(GatewayError::InsufficientNodeBalance { account, balance });
            }
        }
        let shared_session_stats = self.session_stats.take().unwrap_or_default();
        let stats_event_sender = self.start_stats_collector(
            shared_session_stats,
            shutdown.fork("statistics::GatewayStatisticsCollector"),
        );

        let topology_provider = self.gateway_topology_provider();

        let handler_config = CredentialHandlerConfig {
            revocation_bandwidth_penalty: self
                .config
                .debug
                .zk_nym_tickets
                .revocation_bandwidth_penalty,
            pending_poller: self.config.debug.zk_nym_tickets.pending_poller,
            minimum_api_quorum: self.config.debug.zk_nym_tickets.minimum_api_quorum,
            minimum_redemption_tickets: self.config.debug.zk_nym_tickets.minimum_redemption_tickets,
            maximum_time_between_redemption: self
                .config
                .debug
                .zk_nym_tickets
                .maximum_time_between_redemption,
        };

        let ecash_verifier = Arc::new(
            EcashManager::new(
                handler_config,
                nyxd_client,
                self.identity_keypair.public_key().to_bytes(),
                shutdown.fork("EcashVerifier"),
                self.client_storage.clone(),
            )
            .await?,
        );

        let mix_forwarding_channel = self.start_packet_forwarder(shutdown.fork("PacketForwarder"));

        let active_clients_store = ActiveClientsStore::new(stats_event_sender.clone());
        self.start_mix_socket_listener(
            mix_forwarding_channel.clone(),
            active_clients_store.clone(),
            shutdown.fork("mixnet_handling::Listener"),
        );

        self.start_client_websocket_listener(
            mix_forwarding_channel.clone(),
            active_clients_store.clone(),
            shutdown.fork("websocket::Listener"),
            ecash_verifier.clone(),
            stats_event_sender.clone(),
        );

        let nr_request_filter = if self.config.network_requester.enabled {
            let embedded_nr = self
                .start_network_requester(
                    mix_forwarding_channel.clone(),
                    topology_provider.clone(),
                    shutdown.fork("NetworkRequester"),
                )
                .await?;
            // insert information about embedded NR to the active clients store
            active_clients_store.insert_embedded(embedded_nr.handle);
            Some(embedded_nr.used_request_filter)
        } else {
            info!("embedded network requester is disabled");
            None
        };

        if self.config.ip_packet_router.enabled {
            let embedded_ip_sp = self
                .start_ip_packet_router(
                    mix_forwarding_channel.clone(),
                    topology_provider.clone(),
                    shutdown.fork("ip_service_provider"),
                )
                .await?;
            active_clients_store.insert_embedded(embedded_ip_sp);
        } else {
            info!("embedded ip packet router is disabled");
        };

        let _wg_api = if self.wireguard_data.is_some() {
            let embedded_auth = self
                .start_authenticator(
                    mix_forwarding_channel,
                    topology_provider,
                    shutdown.fork("authenticator"),
                    ecash_verifier,
                )
                .await
                .map_err(|source| GatewayError::AuthenticatorStartError { source })?;
            active_clients_store.insert_embedded(embedded_auth.handle);
            Some(embedded_auth.wg_api)
        } else {
            None
        };

        if self.run_http_server {
            HttpApiBuilder::new(
                &self.config,
                self.identity_keypair.as_ref(),
                self.sphinx_keypair.clone(),
            )
            .with_maybe_network_requester(self.network_requester_opts.as_ref().map(|o| &o.config))
            .with_maybe_network_request_filter(nr_request_filter)
            .with_maybe_ip_packet_router(self.ip_packet_router_opts.as_ref().map(|o| &o.config))
            .start(shutdown.fork("http-api"))?;
        }

        info!("Finished nym gateway startup procedure - it should now be able to receive mix and client traffic!");

        info!(
            "Public key: {:?}",
            self.identity_keypair.public_key().to_string()
        );

        if let Err(source) = shutdown.wait_for_shutdown().await {
            // that's a nasty workaround, but anyhow errors are generally nicer, especially on exit
            return Err(GatewayError::ShutdownFailure { source });
        }

        Ok(())
    }
}

// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use self::helpers::load_ip_packet_router_config;
use self::storage::PersistentStorage;
use crate::commands::helpers::{
    override_ip_packet_router_config, override_network_requester_config,
    OverrideIpPacketRouterConfig, OverrideNetworkRequesterConfig,
};
use crate::config::Config;
use crate::error::GatewayError;
use crate::http::HttpApiBuilder;
use crate::node::client_handling::active_clients::ActiveClientsStore;
use crate::node::client_handling::embedded_clients::{LocalEmbeddedClientHandle, MessageRouter};
use crate::node::client_handling::websocket;
use crate::node::client_handling::websocket::connection_handler::coconut::CoconutVerifier;
use crate::node::helpers::{initialise_main_storage, load_network_requester_config};
use crate::node::mixnet_handling::receiver::connection_handler::ConnectionHandler;
use crate::node::statistics::collector::GatewayStatisticsCollector;
use crate::node::storage::Storage;
use anyhow::bail;
use dashmap::DashMap;
use futures::channel::{mpsc, oneshot};
use log::*;
use nym_crypto::asymmetric::{encryption, identity};
use nym_mixnet_client::forwarder::{MixForwardingSender, PacketForwarder};
use nym_network_defaults::NymNetworkDetails;
use nym_network_requester::{LocalGateway, NRServiceProviderBuilder, RequestFilter};
use nym_node::wireguard::types::GatewayClientRegistry;
use nym_statistics_common::collector::StatisticsSender;
use nym_task::{TaskClient, TaskManager};
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::error::Error;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) mod client_handling;
pub(crate) mod helpers;
pub(crate) mod mixnet_handling;
pub(crate) mod statistics;
pub(crate) mod storage;

// TODO: should this struct live here?
struct StartedNetworkRequester {
    /// Request filter, either an exit policy or the allow list, used by the network requester.
    used_request_filter: RequestFilter,

    /// Handle to interact with the local network requester
    handle: LocalEmbeddedClientHandle,
}

/// Wire up and create Gateway instance
pub(crate) async fn create_gateway(
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

    let nr_opts = network_requester_config.map(|config| LocalNetworkRequesterOpts {
        config: config.clone(),
        custom_mixnet_path: custom_mixnet.clone(),
    });

    let ip_opts = ip_packet_router_config.map(|config| LocalIpPacketRouterOpts {
        config,
        custom_mixnet_path: custom_mixnet,
    });

    Gateway::new(config, nr_opts, ip_opts, storage)
}

#[derive(Debug, Clone)]
pub struct LocalNetworkRequesterOpts {
    config: nym_network_requester::Config,

    custom_mixnet_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct LocalIpPacketRouterOpts {
    config: nym_ip_packet_router::Config,

    custom_mixnet_path: Option<PathBuf>,
}

pub(crate) struct Gateway<St = PersistentStorage> {
    config: Config,

    network_requester_opts: Option<LocalNetworkRequesterOpts>,

    ip_packet_router_opts: Option<LocalIpPacketRouterOpts>,

    /// ed25519 keypair used to assert one's identity.
    identity_keypair: Arc<identity::KeyPair>,

    /// x25519 keypair used for Diffie-Hellman. Currently only used for sphinx key derivation.
    sphinx_keypair: Arc<encryption::KeyPair>,
    storage: St,

    client_registry: Arc<GatewayClientRegistry>,
}

impl<St> Gateway<St> {
    /// Construct from the given `Config` instance.
    pub fn new(
        config: Config,
        network_requester_opts: Option<LocalNetworkRequesterOpts>,
        ip_packet_router_opts: Option<LocalIpPacketRouterOpts>,
        storage: St,
    ) -> Result<Self, GatewayError> {
        Ok(Gateway {
            storage,
            identity_keypair: Arc::new(helpers::load_identity_keys(&config)?),
            sphinx_keypair: Arc::new(helpers::load_sphinx_keys(&config)?),
            config,
            network_requester_opts,
            ip_packet_router_opts,
            client_registry: Arc::new(DashMap::new()),
        })
    }

    #[cfg(test)]
    pub async fn new_from_keys_and_storage(
        config: Config,
        network_requester_opts: Option<LocalNetworkRequesterOpts>,
        ip_packet_router_opts: Option<LocalIpPacketRouterOpts>,
        identity_keypair: identity::KeyPair,
        sphinx_keypair: encryption::KeyPair,
        storage: St,
    ) -> Self {
        Gateway {
            config,
            network_requester_opts,
            ip_packet_router_opts,
            identity_keypair: Arc::new(identity_keypair),
            sphinx_keypair: Arc::new(sphinx_keypair),
            storage,
            client_registry: Arc::new(DashMap::new()),
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

    #[cfg(all(feature = "wireguard", target_os = "linux"))]
    async fn start_wireguard(
        &self,
        shutdown: TaskClient,
    ) -> Result<defguard_wireguard_rs::WGApi, Box<dyn Error + Send + Sync>> {
        nym_wireguard::start_wireguard(shutdown, Arc::clone(&self.client_registry)).await
    }

    #[cfg(all(feature = "wireguard", not(target_os = "linux")))]
    async fn start_wireguard(&self, _shutdown: TaskClient) {
        nym_wireguard::start_wireguard().await
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
    async fn start_network_requester(
        &self,
        forwarding_channel: MixForwardingSender,
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

    async fn wait_for_interrupt(
        mut shutdown: TaskManager,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
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

    pub async fn run(self) -> anyhow::Result<()>
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
            CoconutVerifier::new(nyxd_client, self.config.gateway.only_coconut_credentials).await
        }?;

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

        self.start_client_websocket_listener(
            mix_forwarding_channel.clone(),
            active_clients_store.clone(),
            shutdown.subscribe().named("websocket::Listener"),
            Arc::new(coconut_verifier),
        );

        let nr_request_filter = if self.config.network_requester.enabled {
            let embedded_nr = self
                .start_network_requester(
                    mix_forwarding_channel.clone(),
                    shutdown.subscribe().named("NetworkRequester"),
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
                    mix_forwarding_channel,
                    shutdown.subscribe().named("ip_service_provider"),
                )
                .await?;
            active_clients_store.insert_embedded(embedded_ip_sp);
        }

        HttpApiBuilder::new(
            &self.config,
            self.identity_keypair.as_ref(),
            self.sphinx_keypair.clone(),
        )
        .with_wireguard_client_registry(self.client_registry.clone())
        .with_maybe_network_requester(self.network_requester_opts.as_ref().map(|o| &o.config))
        .with_maybe_network_request_filter(nr_request_filter)
        .with_maybe_ip_packet_router(self.ip_packet_router_opts.as_ref().map(|o| &o.config))
        .start(shutdown.subscribe().named("http-api"))?;

        // Once this is a bit more mature, make this a commandline flag instead of a compile time
        // flag
        #[cfg(all(feature = "wireguard", target_os = "linux"))]
        let wg_api = self
            .start_wireguard(shutdown.subscribe().named("wireguard"))
            .await
            .ok();

        #[cfg(all(feature = "wireguard", not(target_os = "linux")))]
        self.start_wireguard(shutdown.subscribe().named("wireguard"))
            .await;

        info!("Finished nym gateway startup procedure - it should now be able to receive mix and client traffic!");

        if let Err(err) = Self::wait_for_interrupt(shutdown).await {
            // that's a nasty workaround, but anyhow errors are generally nicer, especially on exit
            bail!("{err}")
        }
        #[cfg(all(feature = "wireguard", target_os = "linux"))]
        if let Some(wg_api) = wg_api {
            defguard_wireguard_rs::WireguardInterfaceApi::remove_interface(&wg_api)?;
        }
        Ok(())
    }
}

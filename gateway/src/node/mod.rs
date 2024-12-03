// Copyright 2020-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::GatewayError;
use crate::node::client_handling::websocket;
use crate::node::internal_service_providers::{
    ExitServiceProviders, SPMessageRouterBuilder, ServiceProviderBeingBuilt,
};
use futures::channel::oneshot;
use nym_authenticator::Authenticator;
use nym_credential_verification::ecash::{
    credential_sender::CredentialHandlerConfig, EcashManager,
};
use nym_crypto::asymmetric::ed25519;
use nym_gateway_storage::models::WireguardPeer;
use nym_ip_packet_router::IpPacketRouter;
use nym_mixnet_client::forwarder::MixForwardingSender;
use nym_network_defaults::NymNetworkDetails;
use nym_network_requester::NRServiceProviderBuilder;
use nym_node_metrics::events::MetricEventsSender;
use nym_task::TaskClient;
use nym_topology::TopologyProvider;
use nym_validator_client::nyxd::{Coin, CosmWasmClient};
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::*;
use zeroize::Zeroizing;

pub(crate) mod client_handling;
mod internal_service_providers;

pub use client_handling::active_clients::ActiveClientsStore;
pub use nym_gateway_stats_storage::PersistentStatsStorage;
pub use nym_gateway_storage::{error::GatewayStorageError, GatewayStorage};
pub use nym_sdk::{NymApiTopologyProvider, NymApiTopologyProviderConfig, UserAgent};

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

pub struct GatewayTasksBuilder {
    config: Config,

    network_requester_opts: Option<LocalNetworkRequesterOpts>,

    ip_packet_router_opts: Option<LocalIpPacketRouterOpts>,

    authenticator_opts: Option<LocalAuthenticatorOpts>,

    // TODO: combine with authenticator, since you have to start both
    wireguard_data: Option<nym_wireguard::WireguardData>,

    /// ed25519 keypair used to assert one's identity.
    identity_keypair: Arc<ed25519::KeyPair>,

    storage: GatewayStorage,

    mix_packet_sender: MixForwardingSender,

    metrics_sender: MetricEventsSender,

    mnemonic: Arc<Zeroizing<bip39::Mnemonic>>,

    shutdown: TaskClient,

    // populated and cached as necessary
    ecash_manager: Option<Arc<EcashManager>>,

    wireguard_peers: Option<Vec<WireguardPeer>>,

    wireguard_networks: Option<Vec<IpAddr>>,
}

impl Drop for GatewayTasksBuilder {
    fn drop(&mut self) {
        // disarm the shutdown as it was already used to construct relevant tasks and we don't want the builder
        // to cause shutdown
        self.shutdown.disarm();
    }
}

impl GatewayTasksBuilder {
    pub fn new(
        config: Config,
        identity: Arc<ed25519::KeyPair>,
        storage: GatewayStorage,
        mix_packet_sender: MixForwardingSender,
        metrics_sender: MetricEventsSender,
        mnemonic: Arc<Zeroizing<bip39::Mnemonic>>,
        shutdown: TaskClient,
    ) -> GatewayTasksBuilder {
        GatewayTasksBuilder {
            config,
            network_requester_opts: None,
            ip_packet_router_opts: None,
            authenticator_opts: None,
            wireguard_data: None,
            identity_keypair: identity,
            storage,
            mix_packet_sender,
            metrics_sender,
            mnemonic,
            shutdown,
            ecash_manager: None,
            wireguard_peers: None,
            wireguard_networks: None,
        }
    }

    pub fn set_network_requester_opts(
        &mut self,
        network_requester_opts: Option<LocalNetworkRequesterOpts>,
    ) {
        self.network_requester_opts = network_requester_opts;
    }

    pub fn set_ip_packet_router_opts(
        &mut self,
        ip_packet_router_opts: Option<LocalIpPacketRouterOpts>,
    ) {
        self.ip_packet_router_opts = ip_packet_router_opts;
    }

    pub fn set_authenticator_opts(&mut self, authenticator_opts: Option<LocalAuthenticatorOpts>) {
        self.authenticator_opts = authenticator_opts;
    }

    pub fn set_wireguard_data(&mut self, wireguard_data: nym_wireguard::WireguardData) {
        self.wireguard_data = Some(wireguard_data)
    }

    // if this is to be used anywhere else, we might need some wrapper around it
    async fn build_nyxd_signing_client(
        &self,
    ) -> Result<DirectSigningHttpRpcNyxdClient, GatewayError> {
        let endpoints = self.config.get_nyxd_urls();
        let validator_nyxd = endpoints
            .choose(&mut thread_rng())
            .ok_or(GatewayError::NoNyxdAvailable)?;

        let network_details = NymNetworkDetails::new_from_env();
        let client_config = nyxd::Config::try_from_nym_network_details(&network_details)?;

        let nyxd_client = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            client_config,
            validator_nyxd.as_ref(),
            (**self.mnemonic).clone(),
        )?;

        let mix_denom_base = nyxd_client.current_chain_details().mix_denom.base.clone();
        let account = nyxd_client.address();
        let balance = nyxd_client
            .get_balance(&account, mix_denom_base.clone())
            .await?
            .unwrap_or(Coin::new(0, mix_denom_base));

        // see if we have at least 1nym (i.e. 1'000'000unym)
        if balance.amount < 1_000_000 {
            // don't allow constructing the client of we have to use zknym and don't have sufficient balance
            if self.config.gateway.enforce_zk_nyms {
                return Err(GatewayError::InsufficientNodeBalance { account, balance });
            }

            // TODO: this has to be enforced **ALL THE TIME in ENTRY mode**,
            // because even if we don't demand zknym, somebody may send them and we need sufficient tokens for
            // transaction fees for submitting redemption proposals
            // but we're not going to introduce this check now as it would break a lot of existing gateways,
            // so for now just log this error
            error!("this gateway ({account}) has insufficient balance for possible zk-nym redemption transaction fees. it only has {balance} available.")
        }

        Ok(nyxd_client)
    }

    async fn build_ecash_manager(&self) -> Result<Arc<EcashManager>, GatewayError> {
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

        let nyxd_client = self.build_nyxd_signing_client().await?;
        let ecash_manager = Arc::new(
            EcashManager::new(
                handler_config,
                nyxd_client,
                self.identity_keypair.public_key().to_bytes(),
                self.shutdown.fork("ecash-manager"),
                self.storage.clone(),
            )
            .await?,
        );
        Ok(ecash_manager)
    }

    async fn ecash_manager(&mut self) -> Result<Arc<EcashManager>, GatewayError> {
        match self.ecash_manager.clone() {
            Some(cached) => Ok(cached),
            None => {
                let manager = self.build_ecash_manager().await?;
                self.ecash_manager = Some(manager.clone());
                Ok(manager)
            }
        }
    }

    pub async fn build_websocket_listener(
        &mut self,
        active_clients_store: ActiveClientsStore,
    ) -> Result<websocket::Listener, GatewayError> {
        let shared_state = websocket::CommonHandlerState {
            ecash_verifier: self.ecash_manager().await?,
            storage: self.storage.clone(),
            local_identity: Arc::clone(&self.identity_keypair),
            only_coconut_credentials: self.config.gateway.enforce_zk_nyms,
            bandwidth_cfg: (&self.config).into(),
            metrics_sender: self.metrics_sender.clone(),
            outbound_mix_sender: self.mix_packet_sender.clone(),
            active_clients_store: active_clients_store.clone(),
        };

        Ok(websocket::Listener::new(
            self.config.gateway.websocket_bind_address,
            shared_state,
            self.shutdown.fork("websocket"),
        ))
    }

    fn build_network_requester(
        &mut self,
        topology_provider: Box<dyn TopologyProvider + Send + Sync>,
    ) -> Result<ServiceProviderBeingBuilt<NRServiceProviderBuilder>, GatewayError> {
        // if network requester is enabled, configuration file must be provided!
        let Some(nr_opts) = &self.network_requester_opts else {
            return Err(GatewayError::UnspecifiedNetworkRequesterConfig);
        };

        let mut message_router_builder = SPMessageRouterBuilder::new(
            *self.identity_keypair.public_key(),
            self.mix_packet_sender.clone(),
            self.shutdown.fork("network-requester-message-router"),
        );
        let transceiver = message_router_builder.gateway_transceiver();

        let (on_start_tx, on_start_rx) = oneshot::channel();
        let mut nr_builder = NRServiceProviderBuilder::new(nr_opts.config.clone())
            .with_shutdown(self.shutdown.fork("network-requester-sp"))
            .with_custom_gateway_transceiver(transceiver)
            .with_wait_for_gateway(true)
            .with_minimum_gateway_performance(0)
            .with_custom_topology_provider(topology_provider)
            .with_on_start(on_start_tx);

        if let Some(custom_mixnet) = &nr_opts.custom_mixnet_path {
            nr_builder = nr_builder.with_stored_topology(custom_mixnet)?
        }

        Ok(ServiceProviderBeingBuilt::new(
            on_start_rx,
            nr_builder,
            message_router_builder,
        ))
    }

    fn build_ip_router(
        &mut self,
        topology_provider: Box<dyn TopologyProvider + Send + Sync>,
    ) -> Result<ServiceProviderBeingBuilt<IpPacketRouter>, GatewayError> {
        let Some(ip_opts) = &self.ip_packet_router_opts else {
            return Err(GatewayError::UnspecifiedIpPacketRouterConfig);
        };

        let mut message_router_builder = SPMessageRouterBuilder::new(
            *self.identity_keypair.public_key(),
            self.mix_packet_sender.clone(),
            self.shutdown.fork("ipr-message-router"),
        );
        let transceiver = message_router_builder.gateway_transceiver();

        let (on_start_tx, on_start_rx) = oneshot::channel();
        let mut ip_packet_router = IpPacketRouter::new(ip_opts.config.clone())
            .with_shutdown(self.shutdown.fork("ipr-sp"))
            .with_custom_gateway_transceiver(Box::new(transceiver))
            .with_wait_for_gateway(true)
            .with_minimum_gateway_performance(0)
            .with_custom_topology_provider(topology_provider)
            .with_on_start(on_start_tx);

        if let Some(custom_mixnet) = &ip_opts.custom_mixnet_path {
            ip_packet_router = ip_packet_router.with_stored_topology(custom_mixnet)?
        }

        Ok(ServiceProviderBeingBuilt::new(
            on_start_rx,
            ip_packet_router,
            message_router_builder,
        ))
    }

    pub fn build_exit_service_providers(
        &mut self,
        // TODO: redesign the trait to allow cloning more easily
        // (or use concrete types)
        nr_topology_provider: Box<dyn TopologyProvider + Send + Sync>,
        ipr_topology_provider: Box<dyn TopologyProvider + Send + Sync>,
    ) -> Result<ExitServiceProviders, GatewayError> {
        Ok(ExitServiceProviders {
            network_requester: self.build_network_requester(nr_topology_provider)?,
            ip_router: self.build_ip_router(ipr_topology_provider)?,
        })
    }

    async fn build_wireguard_peers_and_networks(
        &self,
    ) -> Result<(Vec<WireguardPeer>, Vec<IpAddr>), GatewayError> {
        let mut used_private_network_ips = vec![];
        let mut all_peers = vec![];
        for wireguard_peer in self.storage.get_all_wireguard_peers().await?.into_iter() {
            let mut peer = defguard_wireguard_rs::host::Peer::try_from(wireguard_peer.clone())?;
            let Some(peer) = peer.allowed_ips.pop() else {
                let peer_identity = &peer.public_key;
                warn!("Peer {peer_identity} has empty allowed ips. It will be removed",);
                self.storage
                    .remove_wireguard_peer(&peer_identity.to_string())
                    .await?;
                continue;
            };
            used_private_network_ips.push(peer.ip);
            all_peers.push(wireguard_peer);
        }

        Ok((all_peers, used_private_network_ips))
    }

    // only used under linux
    #[allow(dead_code)]
    async fn get_wireguard_peers(&mut self) -> Result<Vec<WireguardPeer>, GatewayError> {
        if let Some(cached) = self.wireguard_peers.take() {
            return Ok(cached);
        }

        let (peers, used_private_network_ips) = self.build_wireguard_peers_and_networks().await?;
        // cache private networks for the other task

        self.wireguard_networks = Some(used_private_network_ips);
        Ok(peers)
    }

    async fn get_wireguard_networks(&mut self) -> Result<Vec<IpAddr>, GatewayError> {
        if let Some(cached) = self.wireguard_networks.take() {
            return Ok(cached);
        }

        let (peers, used_private_network_ips) = self.build_wireguard_peers_and_networks().await?;
        // cache peers for the other task

        self.wireguard_peers = Some(peers);
        Ok(used_private_network_ips)
    }

    pub async fn build_wireguard_authenticator(
        &mut self,
        topology_provider: Box<dyn TopologyProvider + Send + Sync>,
    ) -> Result<ServiceProviderBeingBuilt<Authenticator>, GatewayError> {
        let ecash_manager = self.ecash_manager().await?;
        let used_private_network_ips = self.get_wireguard_networks().await?;

        let Some(opts) = &self.authenticator_opts else {
            return Err(GatewayError::UnspecifiedAuthenticatorConfig);
        };
        let Some(wireguard_data) = &self.wireguard_data else {
            return Err(GatewayError::InternalWireguardError(
                "wireguard not set".to_string(),
            ));
        };

        let mut message_router_builder = SPMessageRouterBuilder::new(
            *self.identity_keypair.public_key(),
            self.mix_packet_sender.clone(),
            self.shutdown.fork("authenticator-message-router"),
        );
        let transceiver = message_router_builder.gateway_transceiver();

        let (on_start_tx, on_start_rx) = oneshot::channel();

        let mut authenticator_server = Authenticator::new(
            opts.config.clone(),
            wireguard_data.inner.clone(),
            used_private_network_ips,
        )
        .with_ecash_verifier(ecash_manager)
        .with_custom_gateway_transceiver(transceiver)
        .with_shutdown(self.shutdown.fork("authenticator-sp"))
        .with_wait_for_gateway(true)
        .with_minimum_gateway_performance(0)
        .with_custom_topology_provider(topology_provider)
        .with_on_start(on_start_tx);

        if let Some(custom_mixnet) = &opts.custom_mixnet_path {
            authenticator_server = authenticator_server.with_stored_topology(custom_mixnet)?
        }

        Ok(ServiceProviderBeingBuilt::new(
            on_start_rx,
            authenticator_server,
            message_router_builder,
        ))
    }

    #[cfg(not(target_os = "linux"))]
    pub async fn try_start_wireguard(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        unimplemented!("wireguard is not supported on this platform")
    }

    #[cfg(target_os = "linux")]
    pub async fn try_start_wireguard(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let all_peers = self.get_wireguard_peers().await?;

        let Some(wireguard_data) = self.wireguard_data.take() else {
            return Err(
                GatewayError::InternalWireguardError("wireguard not set".to_string()).into(),
            );
        };

        nym_wireguard::start_wireguard(
            self.storage.clone(),
            all_peers,
            self.shutdown.fork("wireguard"),
            wireguard_data,
        )
        .await?;
        Ok(())
    }
}

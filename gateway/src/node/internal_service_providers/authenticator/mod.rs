// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::internal_service_providers::authenticator::error::AuthenticatorError;
use futures::channel::oneshot;
use ipnetwork::IpNetwork;
use nym_client_core::{HardcodedTopologyProvider, TopologyProvider};
use nym_sdk::{mixnet::Recipient, GatewayTransceiver};
use nym_task::ShutdownTracker;
use nym_wireguard::WireguardGatewayData;
use std::{net::IpAddr, path::Path, sync::Arc, time::SystemTime};

pub use config::Config;
use nym_credential_verification::upgrade_mode::UpgradeModeDetails;

pub mod config;
pub mod error;
pub mod mixnet_client;
pub mod mixnet_listener;
mod peer_manager;
mod seen_credential_cache;

pub struct OnStartData {
    // to add more fields as required
    pub address: Recipient,
}

impl OnStartData {
    pub fn new(address: Recipient) -> Self {
        Self { address }
    }
}

pub struct Authenticator {
    #[allow(unused)]
    config: Config,
    upgrade_mode_state: UpgradeModeDetails,
    wait_for_gateway: bool,
    custom_topology_provider: Option<Box<dyn TopologyProvider + Send + Sync>>,
    custom_gateway_transceiver: Option<Box<dyn GatewayTransceiver + Send + Sync>>,
    wireguard_gateway_data: WireguardGatewayData,
    ecash_verifier: Arc<dyn nym_credential_verification::ecash::traits::EcashManager + Send + Sync>,
    used_private_network_ips: Vec<IpAddr>,
    shutdown: ShutdownTracker,
    on_start: Option<oneshot::Sender<OnStartData>>,
}

impl Authenticator {
    pub fn new(
        config: Config,
        upgrade_mode_state: UpgradeModeDetails,
        wireguard_gateway_data: WireguardGatewayData,
        used_private_network_ips: Vec<IpAddr>,
        ecash_verifier: Arc<
            dyn nym_credential_verification::ecash::traits::EcashManager + Send + Sync,
        >,
        shutdown: ShutdownTracker,
    ) -> Self {
        Self {
            config,
            upgrade_mode_state,
            wait_for_gateway: false,
            custom_topology_provider: None,
            custom_gateway_transceiver: None,
            ecash_verifier,
            wireguard_gateway_data,
            used_private_network_ips,
            shutdown,
            on_start: None,
        }
    }

    #[must_use]
    #[allow(unused)]
    pub fn with_wait_for_gateway(mut self, wait_for_gateway: bool) -> Self {
        self.wait_for_gateway = wait_for_gateway;
        self
    }

    #[must_use]
    pub fn with_minimum_gateway_performance(mut self, minimum_gateway_performance: u8) -> Self {
        self.config.base.debug.topology.minimum_gateway_performance = minimum_gateway_performance;
        self
    }

    #[must_use]
    #[allow(unused)]
    pub fn with_on_start(mut self, on_start: oneshot::Sender<OnStartData>) -> Self {
        self.on_start = Some(on_start);
        self
    }

    #[must_use]
    #[allow(unused)]
    pub fn with_custom_gateway_transceiver(
        mut self,
        gateway_transceiver: Box<dyn GatewayTransceiver + Send + Sync>,
    ) -> Self {
        self.custom_gateway_transceiver = Some(gateway_transceiver);
        self
    }

    #[must_use]
    #[allow(unused)]
    pub fn with_custom_topology_provider(
        mut self,
        topology_provider: Box<dyn TopologyProvider + Send + Sync>,
    ) -> Self {
        self.custom_topology_provider = Some(topology_provider);
        self
    }

    pub fn with_stored_topology<P: AsRef<Path>>(
        mut self,
        file: P,
    ) -> Result<Self, AuthenticatorError> {
        self.custom_topology_provider =
            Some(Box::new(HardcodedTopologyProvider::new_from_file(file)?));
        Ok(self)
    }

    pub async fn run_service_provider(self) -> Result<(), AuthenticatorError> {
        // Used to notify tasks to shutdown. Not all tasks fully supports this (yet).

        // Connect to the mixnet
        let mixnet_client = crate::node::internal_service_providers::authenticator::mixnet_client::create_mixnet_client(
            &self.config.base,
            self.shutdown.clone(),
            self.custom_gateway_transceiver,
            self.custom_topology_provider,
            self.wait_for_gateway,
            &self.config.storage_paths.common_paths,
        )
            .await?;

        let self_address = *mixnet_client.nym_address();

        let used_private_network_ips =
            std::collections::BTreeSet::from_iter(self.used_private_network_ips.iter());
        let private_ip_network = IpNetwork::new(
            self.config.authenticator.private_ipv4.into(),
            self.config.authenticator.private_network_prefix_v4,
        )?;
        let now = SystemTime::now();
        let free_private_network_ips = private_ip_network
            .iter()
            .map(|ip: IpAddr| {
                if used_private_network_ips.contains(&ip) {
                    (ip.into(), Some(now))
                } else {
                    (ip.into(), None)
                }
            })
            .collect();
        let mixnet_listener = crate::node::internal_service_providers::authenticator::mixnet_listener::MixnetListener::new(
            self.config,
            free_private_network_ips,
            self.wireguard_gateway_data,
            mixnet_client,
            self.upgrade_mode_state,
            self.ecash_verifier,
        );

        tracing::info!("The address of this client is: {self_address}");
        tracing::info!("All systems go. Press CTRL-C to stop the server.");

        if let Some(on_start) = self.on_start {
            if on_start.send(OnStartData::new(self_address)).is_err() {
                // the parent has dropped the channel before receiving the response
                return Err(AuthenticatorError::DisconnectedParent);
            }
        }

        mixnet_listener
            .run(self.shutdown.clone_shutdown_token())
            .await
    }
}

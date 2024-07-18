// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;

use futures::channel::oneshot;
use ipnetwork::IpNetwork;
use nym_client_core::{HardcodedTopologyProvider, TopologyProvider};
use nym_sdk::{mixnet::Recipient, GatewayTransceiver};
use nym_task::{TaskClient, TaskHandle};
use nym_wireguard::{peer_controller::PeerControlResponse, WireguardGatewayData};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{config::Config, error::AuthenticatorError};

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
    wait_for_gateway: bool,
    custom_topology_provider: Option<Box<dyn TopologyProvider + Send + Sync>>,
    custom_gateway_transceiver: Option<Box<dyn GatewayTransceiver + Send + Sync>>,
    wireguard_gateway_data: WireguardGatewayData,
    response_rx: UnboundedReceiver<PeerControlResponse>,
    shutdown: Option<TaskClient>,
    on_start: Option<oneshot::Sender<OnStartData>>,
}

impl Authenticator {
    pub fn new(
        config: Config,
        wireguard_gateway_data: WireguardGatewayData,
        response_rx: UnboundedReceiver<PeerControlResponse>,
    ) -> Self {
        Self {
            config,
            wait_for_gateway: false,
            custom_topology_provider: None,
            custom_gateway_transceiver: None,
            wireguard_gateway_data,
            response_rx,
            shutdown: None,
            on_start: None,
        }
    }

    #[must_use]
    #[allow(unused)]
    pub fn with_shutdown(mut self, shutdown: TaskClient) -> Self {
        self.shutdown = Some(shutdown);
        self
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

        let task_handle: TaskHandle = self.shutdown.map(Into::into).unwrap_or_default();

        // Connect to the mixnet
        let mixnet_client = crate::mixnet_client::create_mixnet_client(
            &self.config.base,
            task_handle.get_handle().named("nym_sdk::MixnetClient"),
            self.custom_gateway_transceiver,
            self.custom_topology_provider,
            self.wait_for_gateway,
            &self.config.storage_paths.common_paths,
        )
        .await?;

        let self_address = *mixnet_client.nym_address();

        let private_ip_network = IpNetwork::new(
            self.config.authenticator.private_ip,
            self.config.authenticator.private_network_prefix,
        )?;
        let mixnet_listener = crate::mixnet_listener::MixnetListener::new(
            self.config,
            private_ip_network,
            self.wireguard_gateway_data,
            self.response_rx,
            mixnet_client,
            task_handle,
        );

        log::info!("The address of this client is: {self_address}");
        log::info!("All systems go. Press CTRL-C to stop the server.");

        if let Some(on_start) = self.on_start {
            if on_start.send(OnStartData::new(self_address)).is_err() {
                // the parent has dropped the channel before receiving the response
                return Err(AuthenticatorError::DisconnectedParent);
            }
        }

        mixnet_listener.run().await
    }
}

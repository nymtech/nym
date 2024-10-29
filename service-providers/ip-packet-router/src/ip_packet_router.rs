#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
#![cfg_attr(not(target_os = "linux"), allow(unused_imports))]

use std::path::Path;

use futures::channel::oneshot;
use nym_client_core::{
    client::mix_traffic::transceiver::GatewayTransceiver, HardcodedTopologyProvider,
    TopologyProvider,
};
use nym_sdk::mixnet::Recipient;
use nym_task::{TaskClient, TaskHandle};

use crate::{
    config::Config,
    error::IpPacketRouterError,
    request_filter::{self, RequestFilter},
};

pub struct OnStartData {
    // to add more fields as required
    pub address: Recipient,

    pub request_filter: RequestFilter,
}

impl OnStartData {
    pub fn new(address: Recipient, request_filter: RequestFilter) -> Self {
        Self {
            address,
            request_filter,
        }
    }
}

pub struct IpPacketRouter {
    #[allow(unused)]
    config: Config,

    wait_for_gateway: bool,
    custom_topology_provider: Option<Box<dyn TopologyProvider + Send + Sync>>,
    custom_gateway_transceiver: Option<Box<dyn GatewayTransceiver + Send + Sync>>,
    shutdown: Option<TaskClient>,
    on_start: Option<oneshot::Sender<OnStartData>>,
}

impl IpPacketRouter {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            wait_for_gateway: false,
            custom_topology_provider: None,
            custom_gateway_transceiver: None,
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
    pub fn with_custom_gateway_transceiver(
        mut self,
        gateway_transceiver: Box<dyn GatewayTransceiver + Send + Sync>,
    ) -> Self {
        self.custom_gateway_transceiver = Some(gateway_transceiver);
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
    ) -> Result<Self, IpPacketRouterError> {
        self.custom_topology_provider =
            Some(Box::new(HardcodedTopologyProvider::new_from_file(file)?));
        Ok(self)
    }

    #[cfg(not(target_os = "linux"))]
    pub async fn run_service_provider(self) -> Result<(), IpPacketRouterError> {
        // for debugging purposes, don't crash in debug builds on non-linux platforms
        if cfg!(debug_assertions) {
            log::error!("ip packet router service provider is not yet supported on this platform");
            Ok(())
        } else {
            todo!("service provider is not yet supported on this platform")
        }
    }

    #[cfg(target_os = "linux")]
    pub async fn run_service_provider(self) -> Result<(), IpPacketRouterError> {
        // Used to notify tasks to shutdown. Not all tasks fully supports this (yet).

        use crate::{mixnet_listener, tun_listener};
        let task_handle: TaskHandle = self.shutdown.map(Into::into).unwrap_or_default();

        // Connect to the mixnet
        let mixnet_client = crate::mixnet_client::create_mixnet_client(
            &self.config.base,
            task_handle.get_handle().named("nym_sdk::MixnetClient[IPR]"),
            self.custom_gateway_transceiver,
            self.custom_topology_provider,
            self.wait_for_gateway,
            &self.config.storage_paths.common_paths,
        )
        .await?;

        let self_address = *mixnet_client.nym_address();

        // Create the TUN device that we interact with the rest of the world with
        let config = nym_tun::tun_device::TunDeviceConfig {
            base_name: crate::constants::TUN_BASE_NAME.to_string(),
            ipv4: crate::constants::TUN_DEVICE_ADDRESS_V4,
            netmaskv4: crate::constants::TUN_DEVICE_NETMASK_V4,
            ipv6: crate::constants::TUN_DEVICE_ADDRESS_V6,
            netmaskv6: crate::constants::TUN_DEVICE_NETMASK_V6.to_string(),
        };
        let (tun_reader, tun_writer) =
            tokio::io::split(nym_tun::tun_device::TunDevice::new_device_only(config)?);

        // Channel used by the IpPacketRouter to signal connected and disconnected clients to the
        // TunListener
        let (connected_clients, connected_clients_rx) = mixnet_listener::ConnectedClients::new();

        let tun_listener = tun_listener::TunListener {
            tun_reader,
            task_client: task_handle.get_handle(),
            connected_clients: connected_clients_rx,
        };
        tun_listener.start();

        let request_filter = request_filter::RequestFilter::new(&self.config).await?;
        request_filter.start_update_tasks().await;

        let mixnet_listener = mixnet_listener::MixnetListener {
            _config: self.config,
            request_filter: request_filter.clone(),
            tun_writer,
            mixnet_client,
            task_handle,
            connected_clients,
        };

        log::info!("The address of this client is: {self_address}");
        log::info!("All systems go. Press CTRL-C to stop the server.");

        if let Some(on_start) = self.on_start {
            if on_start
                .send(OnStartData::new(self_address, request_filter))
                .is_err()
            {
                // the parent has dropped the channel before receiving the response
                return Err(IpPacketRouterError::DisconnectedParent);
            }
        }

        mixnet_listener.run().await
    }
}

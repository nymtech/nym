use std::path::Path;

use error::IpForwarderError;
use futures::{channel::oneshot, StreamExt};
use nym_client_core::{
    client::mix_traffic::transceiver::GatewayTransceiver,
    config::disk_persistence::CommonClientPaths, HardcodedTopologyProvider, TopologyProvider,
};
use nym_sdk::{mixnet::Recipient, NymNetworkDetails};
use nym_sphinx::receiver::ReconstructedMessage;
use nym_task::{TaskClient, TaskHandle};

use crate::config::BaseClientConfig;

pub use crate::config::Config;

pub mod config;
pub mod error;

pub struct OnStartData {
    // to add more fields as required
    pub address: Recipient,
}

impl OnStartData {
    pub fn new(address: Recipient) -> Self {
        Self { address }
    }
}

pub struct IpForwarderBuilder {
    config: Config,
    wait_for_gateway: bool,
    custom_topology_provider: Option<Box<dyn TopologyProvider + Send + Sync>>,
    custom_gateway_transceiver: Option<Box<dyn GatewayTransceiver + Send + Sync>>,
    shutdown: Option<TaskClient>,
    on_start: Option<oneshot::Sender<OnStartData>>,
}

impl IpForwarderBuilder {
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
    pub fn with_shutdown(mut self, shutdown: TaskClient) -> Self {
        self.shutdown = Some(shutdown);
        self
    }

    #[must_use]
    pub fn with_custom_gateway_transceiver(
        mut self,
        gateway_transceiver: Box<dyn GatewayTransceiver + Send + Sync>,
    ) -> Self {
        self.custom_gateway_transceiver = Some(gateway_transceiver);
        self
    }

    #[must_use]
    pub fn with_wait_for_gateway(mut self, wait_for_gateway: bool) -> Self {
        self.wait_for_gateway = wait_for_gateway;
        self
    }

    #[must_use]
    pub fn with_on_start(mut self, on_start: oneshot::Sender<OnStartData>) -> Self {
        self.on_start = Some(on_start);
        self
    }

    #[must_use]
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
    ) -> Result<Self, IpForwarderError> {
        self.custom_topology_provider =
            Some(Box::new(HardcodedTopologyProvider::new_from_file(file)?));
        Ok(self)
    }

    pub async fn run_service_provider(self) -> Result<(), IpForwarderError> {
        // Used to notify tasks to shutdown. Not all tasks fully supports this (yet).
        let task_handle: TaskHandle = self.shutdown.map(Into::into).unwrap_or_default();

        // Connect to the mixnet
        let mixnet_client = create_mixnet_client(
            &self.config.base,
            task_handle.get_handle().named("nym_sdk::MixnetClient"),
            self.custom_gateway_transceiver,
            self.custom_topology_provider,
            self.wait_for_gateway,
            &self.config.storage_paths.common_paths,
        )
        .await?;

        let self_address = *mixnet_client.nym_address();

        // Create the TUN device that we interact with the rest of the world with
        let (tun, tun_task_tx, tun_task_response_rx) =
            nym_wireguard::tun_device::TunDevice::new(None);

        let ip_forwarder_service = IpForwarderService {
            config: self.config,
            tun,
            tun_task_tx,
            tun_task_response_rx,
            mixnet_client,
            task_handle,
        };

        log::info!("The address of this client is: {self_address}");
        log::info!("All systems go. Press CTRL-C to stop the server.");

        if let Some(on_start) = self.on_start {
            if on_start.send(OnStartData::new(self_address)).is_err() {
                // the parent has dropped the channel before receiving the response
                return Err(IpForwarderError::DisconnectedParent);
            }
        }

        ip_forwarder_service.run().await
    }
}

struct IpForwarderService {
    config: Config,
    tun: nym_wireguard::tun_device::TunDevice,
    tun_task_tx: nym_wireguard::tun_task_channel::TunTaskTx,
    tun_task_response_rx: nym_wireguard::tun_task_channel::TunTaskResponseRx,
    mixnet_client: nym_sdk::mixnet::MixnetClient,
    task_handle: TaskHandle,
}

impl IpForwarderService {
    async fn run(mut self) -> Result<(), IpForwarderError> {
        let mut task_client = self.task_handle.fork("main_loop");
        while !task_client.is_shutdown() {
            tokio::select! {
                _ = task_client.recv() => {
                    log::debug!("IpForwarderService [main loop]: received shutdown");
                },
                msg = self.mixnet_client.next() => match msg {
                    Some(msg) => self.on_message(msg).await,
                    None => {
                        log::trace!("IpForwarderService [main loop]: stopping since channel closed");
                        break;
                    },
                }
            }
        }
        log::info!("IpForwarderService: stopping");
        Ok(())
    }

    async fn on_message(&mut self, reconstructed: ReconstructedMessage) {
        todo!();
    }
}

// Helper function to create the mixnet client.
// This is NOT in the SDK since we don't want to expose any of the client-core config types.
// We could however consider moving it to a crate in common in the future.
// TODO: refactor this function and its arguments
async fn create_mixnet_client(
    config: &BaseClientConfig,
    shutdown: TaskClient,
    custom_transceiver: Option<Box<dyn GatewayTransceiver + Send + Sync>>,
    custom_topology_provider: Option<Box<dyn TopologyProvider + Send + Sync>>,
    wait_for_gateway: bool,
    paths: &CommonClientPaths,
) -> Result<nym_sdk::mixnet::MixnetClient, IpForwarderError> {
    let debug_config = config.debug;

    let storage_paths = nym_sdk::mixnet::StoragePaths::from(paths.clone());

    let mut client_builder =
        nym_sdk::mixnet::MixnetClientBuilder::new_with_default_storage(storage_paths)
            .await
            .map_err(|err| IpForwarderError::FailedToSetupMixnetClient { source: err })?
            .network_details(NymNetworkDetails::new_from_env())
            .debug_config(debug_config)
            .custom_shutdown(shutdown)
            .with_wait_for_gateway(wait_for_gateway);
    if !config.get_disabled_credentials_mode() {
        client_builder = client_builder.enable_credentials_mode();
    }
    if let Some(gateway_transceiver) = custom_transceiver {
        client_builder = client_builder.custom_gateway_transceiver(gateway_transceiver);
    }
    if let Some(topology_provider) = custom_topology_provider {
        client_builder = client_builder.custom_topology_provider(topology_provider);
    }

    let mixnet_client = client_builder
        .build()
        .map_err(|err| IpForwarderError::FailedToSetupMixnetClient { source: err })?;

    mixnet_client
        .connect_to_mixnet()
        .await
        .map_err(|err| IpForwarderError::FailedToConnectToMixnet { source: err })
}

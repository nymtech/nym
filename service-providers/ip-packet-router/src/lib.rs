#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
#![cfg_attr(not(target_os = "linux"), allow(unused_imports))]

use std::time::Duration;

use error::IpPacketRouterError;

use nym_client_core::{
    client::mix_traffic::transceiver::GatewayTransceiver,
    config::disk_persistence::CommonClientPaths, TopologyProvider,
};

use nym_sdk::NymNetworkDetails;

use nym_task::TaskClient;

use crate::config::BaseClientConfig;

pub use crate::config::Config;
pub use ip_packet_router::{IpPacketRouterBuilder, OnStartData};

pub mod config;
pub mod error;
mod generate_new_ip;
mod ip_packet_router;
mod parse_ip;
mod request_filter;
mod tun_listener;

// The interface used to route traffic
pub const TUN_BASE_NAME: &str = "nymtun";
pub const TUN_DEVICE_ADDRESS: &str = "10.0.0.1";
pub const TUN_DEVICE_NETMASK: &str = "255.255.255.0";

const DISCONNECT_TIMER_INTERVAL: Duration = Duration::from_secs(10);
const CLIENT_INACTIVITY_TIMEOUT: Duration = Duration::from_secs(5 * 60);

// Helper function to create the mixnet client.
// This is NOT in the SDK since we don't want to expose any of the client-core config types.
// We could however consider moving it to a crate in common in the future.
// TODO: refactor this function and its arguments
#[allow(unused)]
async fn create_mixnet_client(
    config: &BaseClientConfig,
    shutdown: TaskClient,
    custom_transceiver: Option<Box<dyn GatewayTransceiver + Send + Sync>>,
    custom_topology_provider: Option<Box<dyn TopologyProvider + Send + Sync>>,
    wait_for_gateway: bool,
    paths: &CommonClientPaths,
) -> Result<nym_sdk::mixnet::MixnetClient, IpPacketRouterError> {
    let debug_config = config.debug;

    let storage_paths = nym_sdk::mixnet::StoragePaths::from(paths.clone());

    let mut client_builder =
        nym_sdk::mixnet::MixnetClientBuilder::new_with_default_storage(storage_paths)
            .await
            .map_err(|err| IpPacketRouterError::FailedToSetupMixnetClient { source: err })?
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
        .map_err(|err| IpPacketRouterError::FailedToSetupMixnetClient { source: err })?;

    mixnet_client
        .connect_to_mixnet()
        .await
        .map_err(|err| IpPacketRouterError::FailedToConnectToMixnet { source: err })
}

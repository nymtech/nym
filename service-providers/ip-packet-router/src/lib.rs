#![cfg_attr(not(target_os = "linux"), allow(dead_code))]
#![cfg_attr(not(target_os = "linux"), allow(unused_imports))]

use std::{net::IpAddr, time::Duration};

use error::IpPacketRouterError;
use futures::{channel::oneshot, StreamExt};
use nym_client_core::{
    client::mix_traffic::transceiver::GatewayTransceiver,
    config::disk_persistence::CommonClientPaths, HardcodedTopologyProvider, TopologyProvider,
};
use nym_ip_packet_requests::{
    DynamicConnectFailureReason, IpPacketRequest, IpPacketRequestData, IpPacketResponse,
    StaticConnectFailureReason,
};
use nym_sdk::{
    mixnet::{InputMessage, MixnetMessageSender, Recipient},
    NymNetworkDetails,
};
use nym_sphinx::receiver::ReconstructedMessage;
use nym_task::{connections::TransmissionLane, TaskClient, TaskHandle};
use request_filter::RequestFilter;
#[cfg(target_os = "linux")]
use tokio::io::AsyncWriteExt;

use crate::{
    config::BaseClientConfig,
    parse_ip::{parse_packet, ParsedPacket},
};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_destination_from_ip_packet() {
        // Create packet
        let builder =
            etherparse::PacketBuilder::ipv4([192, 168, 1, 1], [192, 168, 1, 2], 20).udp(21, 1234);
        let payload = [1, 2, 3, 4, 5, 6, 7, 8];
        let mut packet = Vec::<u8>::with_capacity(builder.size(payload.len()));
        builder.write(&mut packet, &payload).unwrap();

        let dst_addr = parse_dst_addr(&packet).unwrap();
        assert_eq!(dst_addr, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)));
    }
}

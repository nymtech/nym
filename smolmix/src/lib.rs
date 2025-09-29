// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

mod bridge;
mod device;
mod error;
mod reqwest_client;
pub mod tls;

pub use bridge::NymIprBridge;
pub use device::NymIprDevice;
pub use error::SmolmixError;
pub use tls::TlsOverTcp;

use nym_ip_packet_requests::IpPair;
use nym_sdk::stream_wrapper::IpMixStream;
use tokio::sync::mpsc;

/// Create a connected smoltcp device and async bridge for the tunneling packets through the
/// Mixnet to remote hosts via an IPR.
///
/// This function handles the complete setup process:
/// - Ensures the IPR stream is connected
/// - Retrieves allocated IP addresses
/// - Creates communication channels
/// - Constructs the device and bridge components
///
/// # Component Interaction
///
/// ```
///                          create_device()
///                                |
///                 +--------------+---------------+
///                 |              |               |
///                 v              v               v
///           NymIprDevice   NymIprBridge      IpPair
///                 |              |            (10.0.x.x)
///                 |              |
///                 +-- channels --+
///                                |
///                                v
///                           IpMixStream
///                                |
///                                v
///                             Mixnet
/// ```
pub async fn create_device(
    mut ipr_stream: IpMixStream,
) -> Result<(NymIprDevice, NymIprBridge, IpPair), SmolmixError> {
    // Ensure the stream is connected
    if !ipr_stream.is_connected() {
        ipr_stream.connect_tunnel().await?;
    }

    // Get the allocated IPs before moving the stream - need these for proper packet creation
    // further 'up' the flow in the code calling this fn (see examples/tcp_connect.rs).
    let allocated_ips = ipr_stream
        .allocated_ips()
        .ok_or(SmolmixError::NotConnected)?
        .clone();

    // Create channels for device <-> bridge communication
    let (tx_to_bridge, tx_from_device) = mpsc::unbounded_channel();
    let (rx_to_device, rx_from_bridge) = mpsc::unbounded_channel();

    // Create device
    let device = NymIprDevice::new(tx_to_bridge, rx_from_bridge);

    // Create bridge (moves ipr_stream)
    let bridge = NymIprBridge::new(ipr_stream, tx_from_device, rx_to_device);

    Ok((device, bridge, allocated_ips))
}

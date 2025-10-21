// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

use crate::error::MixtcpError;
use nym_ip_packet_requests::codec::MultiIpPacketCodec;
use nym_sdk::stream_wrapper::IpMixStream;
use tokio::sync::mpsc;
use tracing::{error, info};

/// Asynchronous bridge between smoltcp device and Mixnet.
///
/// This component runs in a separate task and handles all asynchronous
/// operations required for outbound communication. It receives packets
/// from the device via channels, bundles them according to IPR protocol
/// (MultiIpPacketCodec) and transmits them through the Mixnet.
///
/// # Packet Processing Flow
///
/// Outgoing packets:
/// - Receive from device via channel
/// - Bundle using MultiIpPacketCodec
/// - Send through mixnet via send_ip_packet()
///
/// Incoming packets:
/// - Poll mixnet with handle_incoming()
/// - Forward to device via channel
/// - Device queues for smoltcp consumption
pub struct NymIprBridge {
    /// Connected IPR stream for mixnet communication
    stream: IpMixStream,
    /// Channel for receiving outgoing packets from device
    tx_receiver: mpsc::UnboundedReceiver<Vec<u8>>,
    /// Channel for sending incoming packets to device
    rx_sender: mpsc::UnboundedSender<Vec<u8>>,
}

impl NymIprBridge {
    pub fn new(
        stream: IpMixStream,
        tx_receiver: mpsc::UnboundedReceiver<Vec<u8>>,
        rx_sender: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Self {
        Self {
            stream,
            tx_receiver,
            rx_sender,
        }
    }

    /// Runs the bridge event loop.
    ///
    /// This method should be spawned in a separate task. It continuously:
    /// - Processes outgoing packets from the device
    /// - Polls for incoming packets from the mixnet
    /// - Maintains packet statistics
    ///
    /// The loop exits when channels are closed or an error occurs.
    pub async fn run(mut self) -> Result<(), MixtcpError> {
        info!("Starting Nym IPR bridge");
        let mut packets_sent = 0;
        let mut packets_received = 0;

        loop {
            tokio::select! {
                // Outgoing packets from smoltcp layer above.
                Some(packet) = self.tx_receiver.recv() => {
                    info!("Bridge sending {} byte packet to mixnet", packet.len());

                    // Log packet details for debugging
                    if packet.len() >= 20 {
                        let version = (packet[0] >> 4) & 0xF;
                        let proto = packet[9];
                        let src_ip = &packet[12..16];
                        let dst_ip = &packet[16..20];
                        info!(
                            "Outgoing IPv{} packet: proto={}, src={}.{}.{}.{}, dst={}.{}.{}.{}",
                            version, proto,
                            src_ip[0], src_ip[1], src_ip[2], src_ip[3],
                            dst_ip[0], dst_ip[1], dst_ip[2], dst_ip[3]
                        );
                    }

                    // Necessary to bundle for IPR! See stream_wrapper_ipr.rs tests.
                    let bundled = MultiIpPacketCodec::bundle_one_packet(packet.into());
                    if let Err(e) = self.stream.send_ip_packet(&bundled).await {
                        error!("Failed to send packet through mixnet: {}", e);
                    } else {
                        packets_sent += 1;
                        info!("Total packets sent: {}", packets_sent);
                    }
                }

                // Poll for incoming packets from mixnet
                Ok(packets) = self.stream.handle_incoming() => {
                    if !packets.is_empty() {
                        info!("Bridge received {} packets from mixnet", packets.len());
                        for packet in packets {
                            info!("Incoming packet: {} bytes", packet.len());

                            // Forward to device via channel
                            if self.rx_sender.send(packet.to_vec()).is_err() {
                                error!("Failed to send packet to device - receiver dropped");
                                return Err(MixtcpError::ChannelClosed);
                            }
                            packets_received += 1;
                            info!("Total packets received: {}", packets_received);
                        }
                    }
                }

                else => {
                    info!("Bridge shutting down. Sent: {}, Received: {}", packets_sent, packets_received);
                    break;
                }
            }
        }

        Ok(())
    }
}

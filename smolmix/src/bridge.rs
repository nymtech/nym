// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

use crate::error::SmolmixError;
use nym_ip_packet_requests::codec::MultiIpPacketCodec;
use nym_sdk::stream_wrapper::IpMixStream;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, trace};

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
    /// Shutdown signal receiver
    shutdown_rx: oneshot::Receiver<()>,
}

/// Handle for signaling the bridge to shut down gracefully.
pub struct BridgeShutdownHandle {
    tx: oneshot::Sender<()>,
}

impl BridgeShutdownHandle {
    /// Signal the bridge to shut down and disconnect from the mixnet.
    pub fn shutdown(self) {
        let _ = self.tx.send(());
    }
}

impl NymIprBridge {
    pub fn new(
        stream: IpMixStream,
        tx_receiver: mpsc::UnboundedReceiver<Vec<u8>>,
        rx_sender: mpsc::UnboundedSender<Vec<u8>>,
    ) -> (Self, BridgeShutdownHandle) {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        (
            Self {
                stream,
                tx_receiver,
                rx_sender,
                shutdown_rx,
            },
            BridgeShutdownHandle { tx: shutdown_tx },
        )
    }

    /// Runs the bridge event loop.
    ///
    /// This method should be spawned in a separate task. It continuously:
    /// - Processes outgoing packets from the device
    /// - Polls for incoming packets from the mixnet
    /// - Maintains packet statistics
    ///
    /// The loop exits when a shutdown signal is received, channels are closed,
    /// or an error occurs. On exit the mixnet client is disconnected gracefully.
    pub async fn run(mut self) -> Result<(), SmolmixError> {
        info!("Starting Nym IPR bridge");
        let mut packets_sent = 0;
        let mut packets_received = 0;

        loop {
            tokio::select! {
                // Shutdown signal
                _ = &mut self.shutdown_rx => {
                    info!(
                        "Bridge received shutdown signal. Packets sent to mixnet: {}, received from mixnet: {}",
                        packets_sent, packets_received
                    );
                    break;
                }

                // Outgoing packets from smoltcp layer above.
                Some(packet) = self.tx_receiver.recv() => {
                    trace!("Bridge sending {} byte packet to mixnet", packet.len());

                    // Necessary to bundle for IPR! See stream_wrapper_ipr.rs tests.
                    let bundled = MultiIpPacketCodec::bundle_one_packet(packet.into());
                    if let Err(e) = self.stream.send_ip_packet(&bundled).await {
                        error!("Failed to send packet through mixnet: {}", e);
                    } else {
                        packets_sent += 1;
                        debug!("Total packets sent: {}", packets_sent);
                    }
                }

                // Poll for incoming packets from mixnet
                Ok(packets) = self.stream.handle_incoming() => {
                    if !packets.is_empty() {
                        trace!("Bridge received {} packets from mixnet", packets.len());
                        for packet in packets {
                            trace!("Incoming packet: {} bytes", packet.len());

                            // Forward to device via channel
                            if self.rx_sender.send(packet.to_vec()).is_err() {
                                error!("Failed to send packet to device - receiver dropped");
                                return Err(SmolmixError::ChannelClosed);
                            }
                            packets_received += 1;
                            debug!("Total packets received: {}", packets_received);
                        }
                    }
                }

                else => {
                    info!(
                        "Bridge shutting down. Packets sent to mixnet: {}, received from mixnet: {}",
                        packets_sent, packets_received
                    );
                    break;
                }
            }
        }

        // Brief delay to let SDK internal tasks drain before teardown.
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        info!("Disconnecting from mixnet...");
        self.stream.disconnect_stream().await;
        info!("Disconnected");

        Ok(())
    }
}

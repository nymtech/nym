// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

use crate::error::SmolmixError;
use nym_ip_packet_requests::codec::MultiIpPacketCodec;
use nym_sdk::stream_wrapper::IpMixStream;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, trace, warn};

/// Asynchronous bridge between the smoltcp device and the Nym mixnet.
///
/// Runs as a background task, shuttling raw IP packets in both directions:
///
/// **Outgoing** (smoltcp → mixnet): receives packets from the device via channel,
/// bundles them with [`MultiIpPacketCodec`] (required by the IPR protocol), and
/// sends them through the mixnet.
///
/// **Incoming** (mixnet → smoltcp): polls the mixnet for packets and forwards
/// them to the device via channel for smoltcp consumption.
pub(crate) struct NymIprBridge {
    stream: IpMixStream,
    /// Receives outgoing packets from the device (smoltcp → bridge → mixnet).
    outgoing_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    /// Sends incoming packets to the device (mixnet → bridge → smoltcp).
    ///
    /// Unbounded: backpressure is handled at the mixnet layer (IPR protocol),
    /// not here. If that changes, consider bounded channels with a drop policy.
    incoming_tx: mpsc::UnboundedSender<Vec<u8>>,
    shutdown_rx: oneshot::Receiver<()>,
}

/// Handle for signaling the bridge to shut down gracefully.
pub(crate) struct BridgeShutdownHandle {
    tx: oneshot::Sender<()>,
}

impl BridgeShutdownHandle {
    pub(crate) fn shutdown(self) {
        let _ = self.tx.send(());
    }
}

impl NymIprBridge {
    pub(crate) fn new(
        stream: IpMixStream,
        outgoing_rx: mpsc::UnboundedReceiver<Vec<u8>>,
        incoming_tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> (Self, BridgeShutdownHandle) {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        (
            Self {
                stream,
                outgoing_rx,
                incoming_tx,
                shutdown_rx,
            },
            BridgeShutdownHandle { tx: shutdown_tx },
        )
    }

    /// Runs the bridge event loop.
    ///
    /// Should be spawned via `tokio::spawn`. The loop exits when a shutdown
    /// signal is received, channels close, or an unrecoverable error occurs.
    ///
    /// # Cancel safety
    ///
    /// `IpMixStream::handle_incoming()` is **not** cancel-safe — its internal
    /// `FramedRead` buffers partial frames, and it mutates connection state after
    /// awaiting. In `tokio::select!`, the shutdown branch can cancel a pending
    /// `handle_incoming()` call, potentially losing buffered data. This is
    /// acceptable during shutdown but worth noting for future changes.
    pub(crate) async fn run(mut self) -> Result<(), SmolmixError> {
        info!("Starting bridge");
        let mut packets_sent: u64 = 0;
        let mut packets_received: u64 = 0;

        loop {
            tokio::select! {
                _ = &mut self.shutdown_rx => {
                    info!(packets_sent, packets_received, "Bridge received shutdown signal");
                    break;
                }

                Some(packet) = self.outgoing_rx.recv() => {
                    trace!(len = packet.len(), "Sending packet to mixnet");

                    // IPR expects packets wrapped in MultiIpPacketCodec framing.
                    let bundled = MultiIpPacketCodec::bundle_one_packet(packet.into());
                    if let Err(e) = self.stream.send_ip_packet(&bundled).await {
                        error!("Failed to send packet through mixnet: {e}");
                    } else {
                        packets_sent += 1;
                        debug!(packets_sent, "Packet sent");
                    }
                }

                result = self.stream.handle_incoming() => {
                    match result {
                        Ok(packets) if !packets.is_empty() => {
                            trace!(count = packets.len(), "Received packets from mixnet");
                            for packet in packets {
                                if self.incoming_tx.send(packet.to_vec()).is_err() {
                                    error!("Device channel closed");
                                    return Err(SmolmixError::ChannelClosed);
                                }
                                packets_received += 1;
                            }
                            debug!(packets_received, "Packets received");
                        }
                        Ok(_) => {} // empty batch, keep polling
                        Err(e) => {
                            // handle_incoming() internally uses a 10-second timeout,
                            // so this won't busy-loop on persistent errors.
                            warn!("Mixnet receive error: {e}");
                        }
                    }
                }

                else => {
                    info!(packets_sent, packets_received, "All channels closed, shutting down");
                    break;
                }
            }
        }

        // disconnect_stream() internally waits for all SDK tasks via TaskTracker.
        info!("Disconnecting from mixnet...");
        self.stream.disconnect_stream().await;
        info!("Disconnected");

        Ok(())
    }
}

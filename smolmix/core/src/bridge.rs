// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

use crate::error::SmolmixError;
use futures::channel::mpsc;
use futures::StreamExt;
use nym_ip_packet_requests::codec::MultiIpPacketCodec;
use nym_sdk::ipr_wrapper::IpMixStream;
use nym_sdk::Error as SdkError;
use std::time::Duration;
use tokio::sync::oneshot;
use tracing::{debug, error, info, trace, warn};

/// Delay before retrying a failed mixnet receive, so a transient error
/// cannot spin the event loop.
const RECEIVE_RETRY_DELAY: Duration = Duration::from_secs(1);

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
    tx: Option<oneshot::Sender<()>>,
}

impl BridgeShutdownHandle {
    /// Signal the bridge to shut down gracefully.
    ///
    /// Sends a one-shot signal that breaks the bridge event loop. The bridge
    /// then calls `IpMixStream::disconnect()` before returning. Consumes
    /// `self`, so can only be called once.
    ///
    /// If the handle is dropped without calling `shutdown()`, the `Drop` impl
    /// sends the signal anyway.
    pub(crate) fn shutdown(mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for BridgeShutdownHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(());
        }
    }
}

impl NymIprBridge {
    /// Create a new bridge and its associated shutdown handle.
    ///
    /// Returns `(bridge, shutdown_handle)`.
    ///
    /// # Parameters
    /// - `stream`: the connected `IpMixStream` (owns the mixnet client)
    /// - `outgoing_rx`: receives raw IP packets from the smoltcp device
    /// - `incoming_tx`: sends raw IP packets to the smoltcp device
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
            BridgeShutdownHandle {
                tx: Some(shutdown_tx),
            },
        )
    }

    /// Runs the bridge event loop, then disconnects the mixnet client.
    ///
    /// Should be spawned via `tokio::spawn`. The loop exits when a shutdown
    /// signal is received or an unrecoverable error occurs; either way the
    /// disconnect below runs, so there is exactly one exit path.
    pub(crate) async fn run(mut self) -> Result<(), SmolmixError> {
        info!("Starting bridge");
        let result = self.event_loop().await;

        // disconnect() internally waits for all SDK tasks via TaskTracker.
        info!("Disconnecting from mixnet...");
        self.stream.disconnect().await;
        info!("Disconnected");
        result
    }

    /// The select loop proper, split out of [`Self::run`] so an error can
    /// simply be returned instead of stashed in a flag for the disconnect
    /// code after the loop.
    ///
    /// # Cancel safety
    ///
    /// Every future raced here is cancel-safe. In particular
    /// `IpMixStream::handle_incoming()` has a single await point (a channel
    /// receive) and mutates no state before it resolves, so losing the race
    /// to another arm cannot drop a buffered packet. The pacing delay uses an
    /// absolute deadline, so a cancelled sleep waits to the same instant next
    /// iteration. It does not restart.
    async fn event_loop(&mut self) -> Result<(), SmolmixError> {
        let mut packets_sent: u64 = 0;
        let mut packets_received: u64 = 0;
        // A deadline set after a transient receive error. The delay is inside
        // the receive arm's own future, so it paces only the receive path:
        // outgoing packets and shutdown stay responsive in their arms while an
        // instantly-returning error cannot spin the loop. The deadline is
        // absolute, so outgoing traffic that cancels the sleep cannot push the
        // retry past RECEIVE_RETRY_DELAY.
        let mut retry_at: Option<tokio::time::Instant> = None;

        loop {
            tokio::select! {
                _ = &mut self.shutdown_rx => {
                    info!(packets_sent, packets_received, "Bridge received shutdown signal");
                    return Ok(());
                }

                // When the device drops its sender this arm yields `None`,
                // stops matching, and the bridge keeps relaying inbound
                // packets until told to shut down.
                Some(packet) = self.outgoing_rx.next() => {
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

                result = async {
                    if let Some(at) = retry_at {
                        tokio::time::sleep_until(at).await;
                    }
                    self.stream.handle_incoming().await
                } => {
                    retry_at = None;
                    match result {
                        Ok(packets) if !packets.is_empty() => {
                            trace!(count = packets.len(), "Received packets from mixnet");
                            for packet in packets {
                                if self.incoming_tx.unbounded_send(packet.to_vec()).is_err() {
                                    error!("Device channel closed");
                                    return Err(SmolmixError::ChannelClosed);
                                }
                                packets_received += 1;
                            }
                            debug!(packets_received, "Packets received");
                        }
                        Ok(_) => {} // empty batch, keep polling
                        Err(e @ (SdkError::IPRClientStreamClosed | SdkError::IprTunnelDisconnected)) => {
                            // The stream is gone and errors return instantly;
                            // retrying would busy-loop.
                            error!("Mixnet receive error, stopping bridge: {e}");
                            return Err(e.into());
                        }
                        Err(e) => {
                            warn!("Mixnet receive error: {e}");
                            retry_at = Some(tokio::time::Instant::now() + RECEIVE_RETRY_DELAY);
                        }
                    }
                }
            }
        }
    }
}

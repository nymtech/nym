// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::listener::received::{MixnetPacketsReceiver, MixnetPacketsSender, ReceivedPacket};
use crate::test_packet::{TestPacketContent, TestPacketHeader};
use anyhow::{Context, bail};
use futures::StreamExt;
use futures::channel::mpsc::unbounded;
use nym_crypto::asymmetric::x25519;
use nym_sphinx_types::{ProcessedPacketData, SphinxPacket};
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

/// A decoded test packet together with its measured round-trip time.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ProcessedPacket {
    /// The packet ID copied from the embedded [`TestPacketContent`].
    pub(crate) id: u64,

    /// Round-trip time measured from when the packet was created to when it was received.
    /// This includes both the sphinx delay and the network transit time; callers should
    /// subtract `config.packet_delay` to obtain the network-only latency.
    pub(crate) rtt: Duration,
}

impl Display for ProcessedPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.id, humantime::format_duration(self.rtt))
    }
}

/// Strategy used to decrypt a returning sphinx packet and extract its [`TestPacketContent`].
///
/// When the agent operates with a reusable header it already holds the payload key, so
/// only the payload needs unwrapping. When it builds a fresh header per-packet the full
/// sphinx processing path (DH + decryption) must be performed using the agent's private key.
pub(crate) enum PayloadRecovery {
    /// The agent holds a pre-built [`TestPacketHeader`] whose payload key can be used to
    /// unwrap the payload directly, skipping the full sphinx processing step.
    ReusableHeader(TestPacketHeader),

    /// The agent must perform full sphinx processing using its private key to decrypt
    /// the payload, as no pre-built header is available.
    FullProcessing(Arc<x25519::KeyPair>),
}

impl From<TestPacketHeader> for PayloadRecovery {
    fn from(header: TestPacketHeader) -> Self {
        PayloadRecovery::ReusableHeader(header)
    }
}

impl From<Arc<x25519::KeyPair>> for PayloadRecovery {
    fn from(private_key: Arc<x25519::KeyPair>) -> Self {
        PayloadRecovery::FullProcessing(private_key)
    }
}

impl PayloadRecovery {
    /// Decrypts `received` and deserialises its payload into a [`TestPacketContent`].
    /// Returns an error if decryption fails or the packet is not addressed to the final hop.
    pub(crate) fn recover_test_payload(
        &self,
        received: SphinxPacket,
    ) -> anyhow::Result<TestPacketContent> {
        match self {
            PayloadRecovery::ReusableHeader(header) => header.recover_payload(received.payload),
            PayloadRecovery::FullProcessing(private_key) => {
                let ProcessedPacketData::FinalHop { payload, .. } =
                    received.process(private_key.private_key().inner())?.data
                else {
                    bail!("received non final hop data")
                };
                TestPacketContent::from_bytes(&payload.recover_plaintext()?)
            }
        }
    }
}

/// Receives raw sphinx packets forwarded by the [`MixnetListener`](crate::listener::MixnetListener),
/// decrypts them, and exposes them as [`ProcessedPacket`]s with RTT measurements.
///
/// The processor owns one half of an unbounded channel; the sender half is cloned and handed
/// to the listener via [`sender`](Self::sender). Packets can be consumed one at a time with
/// [`next_packet`](Self::next_packet) or drained in bulk with [`all_available`](Self::all_available).
pub(crate) struct MixnetPacketProcessor {
    /// Decryption strategy: either reuse a pre-built header or perform full sphinx processing.
    payload_recovery: PayloadRecovery,

    /// How long [`next_packet`](Self::next_packet) will wait before returning a timeout error.
    receive_timeout: Duration,

    /// Sender half kept alive so the channel stays open as long as the processor exists.
    sender: MixnetPacketsSender,

    /// Receiver half polled by [`next_packet`](Self::next_packet) and [`all_available`](Self::all_available).
    receiver: MixnetPacketsReceiver,
}

impl MixnetPacketProcessor {
    /// Creates a new processor along with an internal channel for receiving packets.
    pub(crate) fn new(payload_recovery: PayloadRecovery, receive_timeout: Duration) -> Self {
        let (sender, receiver) = unbounded();

        Self {
            payload_recovery,
            receive_timeout,
            sender,
            receiver,
        }
    }

    /// Returns a clone of the sender half so the listener can forward packets to this processor.
    pub(crate) fn sender(&self) -> MixnetPacketsSender {
        self.sender.clone()
    }

    /// Decrypts a [`ReceivedPacket`] and computes its RTT from the embedded send timestamp.
    fn process_received(&self, packet: ReceivedPacket) -> anyhow::Result<ProcessedPacket> {
        let sphinx_packet = packet
            .received
            .into_inner()
            .to_sphinx_packet()
            .context("the received packet was not a sphinx packet!")?;
        let received_content = self.payload_recovery.recover_test_payload(sphinx_packet)?;
        let latency = packet.received_at - received_content.sending_timestamp;

        Ok(ProcessedPacket {
            id: received_content.id,
            rtt: latency.unsigned_abs(),
        })
    }

    /// Drains all packets currently available in the channel without blocking.
    /// Returns a vec of results — decryption failures are included as `Err` entries rather
    /// than causing the entire drain to abort.
    pub(crate) fn all_available(&mut self) -> Vec<anyhow::Result<ProcessedPacket>> {
        let mut packets = Vec::new();
        while let Ok(Some(pending)) = self.receiver.try_next() {
            packets.push(self.process_received(pending));
        }

        packets
    }

    /// Waits for the next packet, up to `receive_timeout`.
    /// Returns `Err` on timeout, channel exhaustion, or decryption failure.
    pub(crate) async fn next_packet(&mut self) -> anyhow::Result<ProcessedPacket> {
        let packet = timeout(self.receive_timeout, self.receiver.next())
            .await?
            .context("stream has been exhausted")?;

        self.process_received(packet)
    }
}

// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Opening a returned test packet and timing it.
//!
//! Deliberately outside the [`mixnet`](crate::mixnet) module: these are properties of the PACKET,
//! not of the wire it travelled over. A gateway probe needs exactly this for both of its phases,
//! including the one whose packets arrive over a client websocket rather than over Noise.

use crate::mixnet::sphinx::test_packet::{TestPacketContent, TestPacketHeader};
use anyhow::bail;
use nym_crypto::asymmetric::x25519;
use nym_sphinx_types::{ProcessedPacketData, SphinxPacket};
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;

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

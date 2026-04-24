// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Packet types and the generic wire-format trait used by the simulation.
//!
//! The central abstraction is [`WirePacketFormat`]: a trait that any packet
//! type must implement to participate in a simulation.  It covers only
//! wire serialisation; mix logic is handled separately by
//! [`nym_lp_data::mixnodes::traits::MixnodeProcessingPipeline`].
//!

pub mod simple;
pub mod sphinx;

/// Trait that every packet type must implement to participate in the simulation.
///
pub trait WirePacketFormat: Sized {
    /// Deserialise a packet from the raw bytes received off the wire.
    ///
    /// # Errors
    ///
    /// Should return an error on length mismatch, invalid magic bytes, or any
    /// other malformed-datagram condition.
    fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self>;

    /// Serialise the packet to its on-wire byte representation, ready to be
    /// sent via UDP.
    fn to_bytes(&self) -> Vec<u8>;
}

impl WirePacketFormat for Vec<u8> {
    fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(bytes.to_vec())
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.clone()
    }
}

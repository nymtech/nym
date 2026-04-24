// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Packet types and the generic wire-format trait used by the simulation.
//!
//! The central abstraction is [`WirePacketFormat`]: a trait that any packet
//! type must implement to participate in a simulation.  It covers only
//! wire serialisation; mix logic is handled separately by
//! [`nym_lp_data::mixnodes::traits::MixnodeProcessingPipeline`].
//!
//! [`SimplePacket`] is a built-in concrete implementation: a fixed-size 64-byte
//! packet (16-byte UUID + 48-byte payload)

use std::fmt::Debug;

mod simple;

pub use simple::{
    SimpleClientUnwrapping, SimpleClientWrappingPipeline, SimpleFrame, SimpleMessage, SimplePacket,
    SimplePassThroughPipeline,
};

/// Trait that every packet type must implement to participate in the simulation.
///
/// ## Bounds
///
/// * `Debug` — required so that [`crate::node::Node::display_state`] can print
///   packet buffers without knowing the concrete type.
/// * `Sized` — required because the trait is used with `Vec<Pkt>` and moved by
///   value in several places.
/// * `Send + 'static` — required because the simulation driver spawns a
///   `tokio::task` that owns the node list.
///
pub trait WirePacketFormat: Debug + Sized + Send + 'static {
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

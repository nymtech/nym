// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Debug, ops::Add, time::Duration};

use nym_common::debug::format_debug_bytes;
use nym_lp_data::{
    AddressedTimedData, TimedData, TimedPayload,
    common::traits::{
        Framing, FramingUnwrap, Transport, TransportUnwrap, WireUnwrappingPipeline,
        WireWrappingPipeline,
    },
};
use nym_sphinx::SphinxPacket;
use rand::Rng;
use rand_distr::{Distribution, Exp};

use crate::{node::NodeId, packet::WirePacketFormat};

/// Newtype wrapper around [`SphinxPacket`] that provides a trimmed [`Debug`]
/// implementation (showing only the first 16 bytes of the serialised form to
/// avoid flooding logs).
pub struct SimSphinxPacket(SphinxPacket);

impl Debug for SimSphinxPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "SphinkPacket {{")?;
        writeln!(f, "    data start:")?;
        for line in format_debug_bytes(&self.0.to_bytes()[..16])?.lines() {
            writeln!(f, "        {line}")?;
        }
        write!(f, "}}")
    }
}

impl WirePacketFormat for SimSphinxPacket {
    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(SimSphinxPacket(SphinxPacket::from_bytes(bytes)?))
    }
}

/// Marker type identifying a fully-unwrapped Sphinx payload.
///
/// Passed through the pipeline's [`FramingUnwrap`] stage so that
/// [`ClientUnwrappingPipeline::process_unwrapped`] can dispatch on the message
/// kind.  In the simulation there is only one kind, so this is a zero-sized
/// unit struct.
///
/// [`FramingUnwrap`]: nym_lp_data::common::traits::FramingUnwrap
/// [`ClientUnwrappingPipeline::process_unwrapped`]: nym_lp_data::clients::traits::ClientUnwrappingPipeline::process_unwrapped
pub struct SphinxMessage;

/// Abstracts adding a Sphinx [`Delay`](nym_sphinx::Delay) to a timestamp type.
pub trait AddDelay: Sized {
    fn add_delay(self, delay: nym_sphinx::Delay) -> Self;
}

impl AddDelay for u32 {
    /// One tick = 1 ms.
    fn add_delay(self, delay: nym_sphinx::Delay) -> Self {
        self + (delay.to_nanos() / 1_000_000) as u32
    }
}

impl AddDelay for std::time::Instant {
    fn add_delay(self, delay: nym_sphinx::Delay) -> Self {
        self + delay.to_duration()
    }
}

/// Timestamp types that can generate Sphinx delays and be advanced by them.
///
/// Implemented for `u32` (discrete ticks, 1 tick = 1 ms) and [`Instant`]
/// (wall-clock time).
pub trait GenerateDelay: Sized + Add<Self::Delay, Output = Self> {
    /// The delay unit that can be added to `Self` (e.g. `u32` ticks or
    /// [`Duration`](std::time::Duration)).
    type Delay;

    /// Draw a per-hop mix delay in milliseconds for inclusion in a Sphinx packet header.
    fn generate_mix_delay(rng: &mut impl Rng) -> u64;

    /// Draw an inter-packet sending delay for the main Poisson loop.
    fn generate_sending_delay(rng: &mut impl Rng) -> Self::Delay;

    /// Draw an inter-packet sending delay for the secondary cover traffic loop.
    fn generate_cover_traffic_delay(rng: &mut impl Rng) -> Self::Delay;
}

impl GenerateDelay for u32 {
    type Delay = u32;

    /// Uniform in `[0, 10]` ms.
    fn generate_mix_delay(rng: &mut impl Rng) -> u64 {
        rng.gen_range(0..=10)
    }

    /// Exponential with mean 10 ticks (ms).
    fn generate_sending_delay(rng: &mut impl Rng) -> u32 {
        // SAFETY : hardcoded > 0 value
        #[allow(clippy::unwrap_used)]
        let exp: Exp<f64> = Exp::new(1.0 / 10.0).unwrap();
        exp.sample(rng).round() as u32
    }

    /// Exponential with mean 100 ticks (ms).
    fn generate_cover_traffic_delay(rng: &mut impl Rng) -> u32 {
        // SAFETY : hardcoded > 0 value
        #[allow(clippy::unwrap_used)]
        let exp: Exp<f64> = Exp::new(1.0 / 100.0).unwrap();
        exp.sample(rng).round() as u32
    }
}

impl GenerateDelay for std::time::Instant {
    type Delay = Duration;

    /// Exponential with mean 50 ms.
    fn generate_mix_delay(rng: &mut impl Rng) -> u64 {
        // SAFETY : hardcoded > 0 value
        #[allow(clippy::unwrap_used)]
        let exp: Exp<f64> = Exp::new(1.0 / 50.0).unwrap();
        exp.sample(rng).round() as u64
    }

    /// Exponential with mean 20 ms.
    fn generate_sending_delay(rng: &mut impl Rng) -> Duration {
        // SAFETY : hardcoded > 0 value
        #[allow(clippy::unwrap_used)]
        let exp: Exp<f64> = Exp::new(1.0 / 20.0).unwrap();
        Duration::from_millis(exp.sample(rng).round() as u64)
    }

    /// Exponential with mean 200 ms.
    fn generate_cover_traffic_delay(rng: &mut impl Rng) -> Duration {
        // SAFETY : hardcoded > 0 value
        #[allow(clippy::unwrap_used)]
        let exp: Exp<f64> = Exp::new(1.0 / 200.0).unwrap();
        Duration::from_millis(exp.sample(rng).round() as u64)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Building blocks

/// Wrapping building block: `SphinxPacket` → `SphinxPacket`.
///
/// Passthrough wrapper and unwrapper for sphinx compatibility demonstration
pub struct SphinxNoOpWireWrapper;

impl<Ts: Clone> Framing<Ts, SphinxPacket> for SphinxNoOpWireWrapper {
    const OVERHEAD_SIZE: usize = 0;
    fn to_frame(
        &self,
        payload: TimedPayload<Ts>,
        _frame_size: usize,
    ) -> Vec<TimedData<Ts, SphinxPacket>> {
        // Since we're passing through, payload shoud already be a single sphinx packet
        // SAFETY: If the pipeline is implemented properly, payload is a correct sphinx packet
        #[allow(clippy::unwrap_used)]
        let sphinx_packet =
            payload.data_transform(|bytes| SphinxPacket::from_bytes(&bytes).unwrap());
        vec![sphinx_packet]
    }
}

impl<Ts: Clone> Transport<Ts, SphinxPacket, SimSphinxPacket, NodeId> for SphinxNoOpWireWrapper {
    const OVERHEAD_SIZE: usize = 0;
    fn to_transport_packet(
        &self,
        frame: TimedData<Ts, SphinxPacket>,
        next_hop: NodeId,
    ) -> AddressedTimedData<Ts, SimSphinxPacket, NodeId> {
        (next_hop, frame.data_transform(SimSphinxPacket)).into()
    }
}

impl<Ts: Clone> WireWrappingPipeline<Ts, SphinxPacket, SimSphinxPacket, NodeId>
    for SphinxNoOpWireWrapper
{
    fn packet_size(&self) -> usize {
        1500
    }
}

/// Unwrapping building block: `SimSphinxPacket` → `SphinxPacket`.
///
/// The inverse of [`SphinxNoOpWireWrapper`]: unwraps the [`SimSphinxPacket`]
/// newtype to the inner [`SphinxPacket`] and serialises it back to bytes for
/// the downstream [`FramingUnwrap`] stage.
///
/// [`FramingUnwrap`]: nym_lp_data::common::traits::FramingUnwrap
pub struct SphinxNoOpWireUnwrapper;

impl<Ts> FramingUnwrap<Ts, SphinxPacket, SphinxMessage> for SphinxNoOpWireUnwrapper {
    fn frame_to_message(
        &mut self,
        frame: TimedData<Ts, SphinxPacket>,
    ) -> Option<(TimedPayload<Ts>, SphinxMessage)> {
        Some((
            frame.data_transform(|sphinx| sphinx.to_bytes()),
            SphinxMessage,
        ))
    }
}

impl<Ts: Clone> TransportUnwrap<Ts, SphinxPacket, SimSphinxPacket> for SphinxNoOpWireUnwrapper {
    fn packet_to_frame(
        &self,
        packet: SimSphinxPacket,
        timestamp: Ts,
    ) -> anyhow::Result<TimedData<Ts, SphinxPacket>> {
        Ok(TimedData {
            timestamp,
            data: packet.0,
        })
    }
}

impl<Ts: Clone> WireUnwrappingPipeline<Ts, SphinxPacket, SimSphinxPacket, SphinxMessage>
    for SphinxNoOpWireUnwrapper
{
}

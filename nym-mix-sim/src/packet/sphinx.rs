// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Debug;

use nym_common::debug::format_debug_bytes;
use nym_lp_data::{
    TimedData, TimedPayload,
    common::traits::{
        Framing, FramingUnwrap, Transport, TransportUnwrap, WireUnwrappingPipeline,
        WireWrappingPipeline,
    },
};
use nym_sphinx::SphinxPacket;

use crate::packet::WirePacketFormat;

// Simple Wrapper for sphinx packet to implement a (sort of) debug impl;
pub struct SimSphinxPacket(SphinxPacket);

impl Debug for SimSphinxPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "SphinkPacket {{")?;
        writeln!(f, "    data start:")?;
        for line in format_debug_bytes(&self.0.to_bytes()[..64])?.lines() {
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

impl<Ts: Clone> Transport<Ts, SphinxPacket, SimSphinxPacket> for SphinxNoOpWireWrapper {
    const OVERHEAD_SIZE: usize = 0;
    fn to_transport_packet(
        &self,
        frame: TimedData<Ts, SphinxPacket>,
    ) -> TimedData<Ts, SimSphinxPacket> {
        frame.data_transform(SimSphinxPacket)
    }
}

impl<Ts: Clone> WireWrappingPipeline<Ts, SphinxPacket, SimSphinxPacket> for SphinxNoOpWireWrapper {
    fn packet_size(&self) -> usize {
        nym_sphinx::params::PacketSize::RegularPacket.size()
    }
}

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

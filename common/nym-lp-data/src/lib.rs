// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Trait definitions and data structures for low-level packet (LP) processing
//! pipelines in the Nym mixnet.
//!
//! ## Crate layout
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`clients`]   | Client-side pipeline traits and types: chunking, reliability, obfuscation, routing security, framing, transport |
//! | [`common`]    | Shared framing and transport traits used by both clients and mixnodes |
//! | [`mixnodes`]  | Mixnode-side pipeline traits: unwrap incoming packets, re-wrap and forward them |
//!
//! ## Core types
//!
//! [`TimedData`] is the foundational wrapper that pairs any piece of data with a
//! timestamp, threading timing information through every stage of the pipeline.
//! [`TimedPayload`] is a convenience alias for `TimedData<Ts, Vec<u8>>`.

use std::fmt::Debug;

pub mod clients;
pub mod common;
pub mod fragmentation;
pub mod mixnodes;
pub mod packet;

/// Convenience alias for [`TimedData`] when the payload is a raw byte buffer.
pub type TimedPayload<Ts> = TimedData<Ts, Vec<u8>>;
/// Convenience alias for [`AddressedTimedData`] when the payload is a raw byte buffer.
pub type AddressedTimedPayload<Ts, NdId> = AddressedTimedData<Ts, Vec<u8>, NdId>;
/// Convenience alias for [`PipelineData`] when the payload is a raw byte buffer.
pub type PipelinePayload<Ts, Opts, NdId> = PipelineData<Ts, Vec<u8>, Opts, NdId>;

/// A value of type `D` tagged with a timestamp of type `Ts`.
///
/// `TimedData` threads timing information through every stage of the LP
/// pipeline.  It is produced by [`clients::traits::Chunking`] and propagated
/// unchanged (or with the timestamp transformed) through every subsequent
/// pipeline stage until the packet is sent on the wire.
#[derive(Clone)]
pub struct TimedData<Ts, D> {
    pub timestamp: Ts,
    pub data: D,
}

impl<Ts, D> Debug for TimedData<Ts, D>
where
    D: Debug,
    Ts: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "TimedData {{")?;
        writeln!(f, "    data:")?;
        let data_debug = format!("{:#?}", &self.data);
        for line in data_debug.lines() {
            writeln!(f, "        {}", line)?;
        }
        writeln!(f, "    timestamp: {:#?},", &self.timestamp)?;
        write!(f, "}}")
    }
}

impl<Ts, D> TimedData<Ts, D> {
    pub fn new(timestamp: Ts, data: D) -> Self {
        TimedData { timestamp, data }
    }
    /// Apply `op` to the data component, leaving the timestamp unchanged.
    ///
    /// `Nd` can differ from `D`, so this also acts as a type transform.
    pub fn data_transform<F, Nd>(self, mut op: F) -> TimedData<Ts, Nd>
    where
        F: FnMut(D) -> Nd,
    {
        TimedData {
            data: op(self.data),
            timestamp: self.timestamp,
        }
    }

    /// Apply `op` to the timestamp component, leaving the data unchanged.
    pub fn ts_transform<F>(self, mut op: F) -> Self
    where
        F: FnMut(Ts) -> Ts,
    {
        TimedData {
            data: self.data,
            timestamp: op(self.timestamp),
        }
    }
}

/// A timestamped payload extended with pipeline-stage options and a destination address.
///
/// `PipelineData` is the value flowing between client-side pipeline stages
/// ([`Chunking`], [`Reliability`], [`Obfuscation`], [`RoutingSecurity`], [`Framing`],
/// [`Transport`]).  It carries:
///
/// - `data`: a [`TimedData`] pairing the payload with its scheduled timestamp,
/// - `options`: per-message configuration consumed by the pipeline (typically an
///   [`InputOptions`] implementor on the client side; `()` once the message is
///   reduced to an addressed payload),
/// - `dst`: the next-hop destination identifier the wire layer should send to.
///
/// [`Chunking`]: crate::clients::traits::Chunking
/// [`Reliability`]: crate::clients::traits::Reliability
/// [`Obfuscation`]: crate::clients::traits::Obfuscation
/// [`RoutingSecurity`]: crate::clients::traits::RoutingSecurity
/// [`Framing`]: crate::common::traits::Framing
/// [`Transport`]: crate::common::traits::Transport
/// [`InputOptions`]: crate::clients::InputOptions
#[derive(Clone)]
pub struct PipelineData<Ts, D, Opts, NdId> {
    pub data: TimedData<Ts, D>,
    pub options: Opts,
    pub dst: NdId,
}

impl<Ts, D, Opts, NdId> PipelineData<Ts, D, Opts, NdId> {
    /// Construct a new [`PipelineData`] from its parts.
    pub fn new(timestamp: Ts, data: D, options: Opts, dst: NdId) -> Self {
        PipelineData {
            data: TimedData::new(timestamp, data),
            options,
            dst,
        }
    }

    /// Apply `op` to the data component, leaving the timestamp, options, and
    /// destination unchanged.
    ///
    /// `Nd` can differ from `D`, so this also acts as a type transform.
    pub fn data_transform<F, Nd>(self, op: F) -> PipelineData<Ts, Nd, Opts, NdId>
    where
        F: FnMut(D) -> Nd,
    {
        PipelineData {
            data: self.data.data_transform(op),
            options: self.options,
            dst: self.dst,
        }
    }

    /// Apply `op` to the timestamp component, leaving the data unchanged.
    pub fn ts_transform<F>(self, op: F) -> Self
    where
        F: FnMut(Ts) -> Ts,
    {
        PipelineData {
            data: self.data.ts_transform(op),
            options: self.options,
            dst: self.dst,
        }
    }

    /// Drop the pipeline options, producing a plain addressed payload.
    pub fn into_addressed(self) -> AddressedTimedData<Ts, D, NdId> {
        AddressedTimedData {
            data: self.data,
            options: (),
            dst: self.dst,
        }
    }
}

impl<Ts, D, Opts, NdId> Debug for PipelineData<Ts, D, Opts, NdId>
where
    D: Debug,
    Ts: Debug,
    NdId: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "PipelineData {{")?;
        writeln!(f, "    dst: {:#?}", &self.dst)?;
        writeln!(f, "    data: {:#?}", &self.data)?;
        write!(f, "}}")
    }
}

/// Convenience alias for [`PipelineData`] when no per-message pipeline options
/// are needed. Avoids duplicating the pipeline data structure.
pub type AddressedTimedData<Ts, D, NdId> = PipelineData<Ts, D, (), NdId>;

impl<Ts, D, NdId> AddressedTimedData<Ts, D, NdId> {
    /// Construct a new [`AddressedTimedData`] with unit `options`.
    pub fn new_addressed(timestamp: Ts, data: D, dst: NdId) -> Self {
        AddressedTimedData {
            data: TimedData::new(timestamp, data),
            options: (),
            dst,
        }
    }
}

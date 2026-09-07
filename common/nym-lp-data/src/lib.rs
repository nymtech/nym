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
//! | [`nymnodes`]  | Mixnode-side pipeline traits: unwrap incoming packets, re-wrap and forward them |
//!
//! ## Core types
//!
//! [`TimedData`] is the foundational wrapper that pairs any piece of data with an
//! [`Instant`] timestamp, threading timing information through every stage of the
//! pipeline. [`TimedPayload`] is a convenience alias for `TimedData<Vec<u8>>`.

use std::net::SocketAddr;
use std::time::Instant;

pub mod clients;
pub mod common;
pub mod fragmentation;
pub mod nymnodes;
pub mod packet;

/// Convenience alias for [`TimedData`] when the payload is a raw byte buffer.
pub type TimedPayload = TimedData<Vec<u8>>;
/// Convenience alias for [`AddressedTimedData`] when the payload is a raw byte buffer.
pub type AddressedTimedPayload = AddressedTimedData<Vec<u8>>;
/// Convenience alias for [`PipelineData`] when the payload is a raw byte buffer.
pub type PipelinePayload<Opts, NdId = SocketAddr> = PipelineData<Vec<u8>, Opts, NdId>;

/// A value of type `D` tagged with an [`Instant`] timestamp.
///
/// `TimedData` threads timing information through every stage of the LP
/// pipeline.  It is produced by [`clients::traits::Chunking`] and propagated
/// unchanged (or with its timestamp replaced via [`TimedData::with_timestamp`])
/// through every subsequent pipeline stage until the packet is sent on the wire.
#[derive(Clone, Debug)]
pub struct TimedData<D> {
    pub timestamp: Instant,
    pub data: D,
}

impl<D> TimedData<D> {
    pub fn new(timestamp: Instant, data: D) -> Self {
        TimedData { timestamp, data }
    }
    /// Apply `op` to the data component, leaving the timestamp unchanged.
    ///
    /// `Nd` can differ from `D`, so this also acts as a type transform.
    pub fn data_transform<F, Nd>(self, mut op: F) -> TimedData<Nd>
    where
        F: FnMut(D) -> Nd,
    {
        TimedData {
            data: op(self.data),
            timestamp: self.timestamp,
        }
    }

    /// Set a new timestamp
    pub fn with_timestamp(self, new_timestamp: Instant) -> Self {
        TimedData {
            data: self.data,
            timestamp: new_timestamp,
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
/// - `options`: opaque per-message metadata threaded through the pipeline (`()`
///   once the message is reduced to an addressed payload),
/// - `dst`: the next-hop socket address the wire layer should send to.
///
/// [`Chunking`]: crate::clients::traits::Chunking
/// [`Reliability`]: crate::clients::traits::Reliability
/// [`Obfuscation`]: crate::clients::traits::Obfuscation
/// [`RoutingSecurity`]: crate::clients::traits::RoutingSecurity
/// [`Framing`]: crate::common::traits::Framing
/// [`Transport`]: crate::common::traits::Transport
#[derive(Clone, Debug)]
pub struct PipelineData<D, Opts, NdId = SocketAddr> {
    pub data: TimedData<D>,
    pub options: Opts,
    pub dst: NdId,
}

impl<D, Opts, NdId> PipelineData<D, Opts, NdId> {
    /// Construct a new [`PipelineData`] from its parts.
    pub fn new(timestamp: Instant, data: D, options: Opts, dst: NdId) -> Self {
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
    pub fn data_transform<F, Nd>(self, op: F) -> PipelineData<Nd, Opts, NdId>
    where
        F: FnMut(D) -> Nd,
    {
        PipelineData {
            data: self.data.data_transform(op),
            options: self.options,
            dst: self.dst,
        }
    }

    /// Set a new timestamp
    pub fn with_timestamp(self, new_timestamp: Instant) -> Self {
        PipelineData {
            data: self.data.with_timestamp(new_timestamp),
            options: self.options,
            dst: self.dst,
        }
    }

    /// Apply `op` to the options component, leaving the timestamp, data, and
    /// destination unchanged.
    ///
    /// `No` can differ from `O`, so this also acts as a type transform.
    pub fn options_transform<F, No>(self, mut op: F) -> PipelineData<D, No, NdId>
    where
        F: FnMut(Opts) -> No,
    {
        PipelineData {
            data: self.data,
            options: op(self.options),
            dst: self.dst,
        }
    }

    /// Set a new destination
    pub fn with_dst<NewNdId>(self, new_dst: NewNdId) -> PipelineData<D, Opts, NewNdId> {
        PipelineData {
            data: self.data,
            options: self.options,
            dst: new_dst,
        }
    }

    /// Drop the pipeline options, producing a plain addressed payload.
    pub fn into_addressed(self) -> AddressedTimedData<D, NdId> {
        AddressedTimedData {
            data: self.data,
            options: (),
            dst: self.dst,
        }
    }
}

/// Convenience alias for [`PipelineData`] when no per-message pipeline options
/// are needed. Avoids duplicating the pipeline data structure.
pub type AddressedTimedData<D, NdId = SocketAddr> = PipelineData<D, (), NdId>;

impl<D, NdId> AddressedTimedData<D, NdId> {
    /// Construct a new [`AddressedTimedData`] with unit `options`.
    pub fn new_addressed(timestamp: Instant, data: D, dst: NdId) -> Self {
        AddressedTimedData {
            data: TimedData::new(timestamp, data),
            options: (),
            dst,
        }
    }

    /// Convert a [`AddressedTimedData`] into a [`PipelineData`] with the provided options.
    pub fn with_options<Opts>(self, opts: Opts) -> PipelineData<D, Opts, NdId> {
        PipelineData {
            data: self.data,
            options: opts,
            dst: self.dst,
        }
    }
}

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
pub mod mixnodes;

/// A value of type `D` tagged with a timestamp of type `Ts`.
///
/// `TimedData` threads timing information through every stage of the LP
/// pipeline.  It is produced by [`clients::traits::Chunking`] and propagated
/// unchanged (or with the timestamp transformed) through every subsequent
/// pipeline stage until the packet is sent on the wire.
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
    /// `Nd` can be a different type to allow type transform as well
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

/// Convenience alias for a [`TimedData`] whose payload is a raw byte buffer.
///
/// Used as the input and output type for most pipeline stages before the data
/// is wrapped into a typed frame or packet.
pub type TimedPayload<Ts> = TimedData<Ts, Vec<u8>>;

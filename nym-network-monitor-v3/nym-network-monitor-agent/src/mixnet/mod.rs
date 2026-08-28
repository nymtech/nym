// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Everything that speaks to a node over its MIXNET listener, which is to say everything that
//! speaks Noise.
//!
//! Grouped by transport rather than by test kind, because the transport is what actually differs
//! between probes. A mixnode probe is wholly within this module; a gateway probe uses it for the leg
//! that sends final-hop packets to the gateway's mixnet listener, and a client websocket session for
//! the rest, which authenticates through the gateway registration handshake and involves no Noise at
//! all. A future protocol leg would be its own module beside this one.
//!
//! Sphinx-level pieces that every transport needs - packet construction, payload recovery, round
//! trip timing - deliberately stay outside: they are properties of the packets, not of the wire they
//! travelled over.

pub(crate) mod demux;
pub(crate) mod egress;
pub(crate) mod events;
pub(crate) mod listener;
pub(crate) mod processor;
pub(crate) mod targets;
#[cfg(test)]
pub(crate) mod test_fixtures;

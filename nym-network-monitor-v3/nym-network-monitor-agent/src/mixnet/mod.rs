// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The mixnet: its packet format, and the wires this agent reaches it over.
//!
//! Sphinx is the mixnet's format, so [`sphinx`] lives here rather than beside this module - a client
//! websocket is not another network, it is another way into this one.
//!
//! Everything else here is the NOISE wire: one shared listener, a handler per inbound connection, the
//! outbound connection, and the per-target routing and inboxes they feed. A gateway probe uses this
//! wire for the leg that sends final-hop packets to a gateway's mixnet listener, and
//! [`client_session`] for the rest, which authenticates through the gateway registration handshake
//! and involves no Noise at all.
//!
//! Now that the second wire is here, these Noise files are worth gathering under a name of their own
//! as well, leaving this module holding one entry per wire plus the format they share. Deliberately
//! not done in the same change as adding the wire, so that the diff which introduces the session is
//! not also the diff which moves seven unrelated files.

pub(crate) mod client_session;
pub(crate) mod connection_handler;
pub(crate) mod egress;
pub(crate) mod events;
pub(crate) mod inbox;
pub(crate) mod listener;
pub(crate) mod sphinx;
pub(crate) mod targets;
#[cfg(test)]
pub(crate) mod test_fixtures;

// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! The mixnet's packet format, and how this agent builds and opens its test packets in it.
//!
//! Separate from the transport siblings beside it because a sphinx packet does not care which wire
//! carried it. A gateway probe proves the point in both directions: its ingress phase builds a packet
//! that leaves over a client websocket and arrives over the mixnet, and its egress phase builds one
//! that leaves over the mixnet and arrives over the websocket. Same packets, same construction, same
//! recovery, two different wires.

pub(crate) mod helpers;
pub(crate) mod payload;
pub(crate) mod test_packet;

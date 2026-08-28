// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use nym_sphinx_framing::packet::FramedNymPacket;
use std::time::Duration;
use time::OffsetDateTime;

/// A sphinx packet received by the [`MixnetListener`](super::MixnetListener), bundled with its
/// wall-clock arrival time.
pub(crate) struct ReceivedPacket {
    /// UTC timestamp at which the packet was pulled off the stream.
    pub(crate) received_at: OffsetDateTime,

    /// The decoded sphinx packet as delivered by the framed codec.
    pub(crate) received: FramedNymPacket,
}

impl ReceivedPacket {
    /// Wraps `received` and stamps it with the current UTC time.
    pub(crate) fn new(received: FramedNymPacket) -> Self {
        Self {
            received_at: OffsetDateTime::now_utc(),
            received,
        }
    }
}

/// Something that happened on ONE target's inbound connection.
///
/// A wave shares a single listener, so a target's own facts have to reach that target rather than a
/// field on the listener: with N targets handshaking against one responder there is nothing for a
/// single `last_noise_handshake_duration` to mean. Carrying them on the target's own channel keeps
/// the ordering honest too, since a handshake necessarily precedes that connection's packets.
pub(crate) enum IngressEvent {
    /// The target connected back and completed the Noise handshake, as timed by the responder.
    ///
    /// Reaching a target's measurement at all implies this arrived, because the probe sequence only
    /// proceeds once a packet has come back and a packet can only come back through a completed
    /// handshake. It is reported rather than assumed so that the plumbing is checkable.
    HandshakeCompleted(Duration),

    /// The target connected back but never got as far as a usable stream.
    ///
    /// Distinguishing this from silence is the diagnostic the shared listener buys: the source is
    /// known at accept time, before the handshake, so a stale or mismatched noise key is
    /// attributable to a node rather than being an anonymous log line.
    HandshakeFailed(String),

    /// A sphinx packet the target returned.
    Packet(ReceivedPacket),
}

/// Sender half of ONE target's ingress channel.
pub(crate) type IngressEventsSender = UnboundedSender<IngressEvent>;

/// Receiver half of ONE target's ingress channel.
pub(crate) type IngressEventsReceiver = UnboundedReceiver<IngressEvent>;

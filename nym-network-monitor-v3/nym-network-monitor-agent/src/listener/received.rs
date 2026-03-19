// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use nym_sphinx_framing::packet::FramedNymPacket;
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

/// Sender half of the channel used to forward [`ReceivedPacket`]s from the listener to the processor.
pub(crate) type MixnetPacketsSender = UnboundedSender<ReceivedPacket>;

/// Receiver half of the channel used to forward [`ReceivedPacket`]s from the listener to the processor.
pub(crate) type MixnetPacketsReceiver = UnboundedReceiver<ReceivedPacket>;

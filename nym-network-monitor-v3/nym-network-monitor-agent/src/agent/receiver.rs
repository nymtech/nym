// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use nym_sphinx_framing::packet::FramedNymPacket;
use time::OffsetDateTime;

pub(crate) struct ReceivedPacket {
    pub(crate) received_at: OffsetDateTime,
    pub(crate) received: FramedNymPacket,
}

impl ReceivedPacket {
    pub(crate) fn new(received: FramedNymPacket) -> Self {
        Self {
            received_at: OffsetDateTime::now_utc(),
            received,
        }
    }
}

pub(crate) type MixnetPacketsSender = UnboundedSender<ReceivedPacket>;
pub(crate) type MixnetPacketsReceiver = UnboundedReceiver<ReceivedPacket>;

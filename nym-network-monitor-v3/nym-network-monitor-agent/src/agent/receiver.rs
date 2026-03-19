// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_sphinx_framing::packet::FramedNymPacket;
use time::OffsetDateTime;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub(crate) struct ReceivedPacket {
    received_at: OffsetDateTime,
    received: FramedNymPacket,
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

pub(crate) struct MixnetPacketsReceiver {
    inner: UnboundedReceiver<ReceivedPacket>,
}

impl MixnetPacketsReceiver {
    pub(crate) fn new(inner: UnboundedReceiver<ReceivedPacket>) -> Self {
        Self { inner }
    }
}

// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;
use futures::channel::mpsc::SendError;
use nym_sphinx::forwarding::packet::MixPacket;
use tokio::time::Instant;

pub fn mix_forwarding_channels() -> (MixForwardingSender, MixForwardingReceiver) {
    let (tx, rx) = mpsc::unbounded();
    (tx.into(), rx)
}

#[derive(Clone)]
pub struct MixForwardingSender(mpsc::UnboundedSender<PacketToForward>);

impl From<mpsc::UnboundedSender<PacketToForward>> for MixForwardingSender {
    fn from(tx: mpsc::UnboundedSender<PacketToForward>) -> Self {
        MixForwardingSender(tx)
    }
}

impl MixForwardingSender {
    pub fn forward_packet(&self, packet: impl Into<PacketToForward>) -> Result<(), SendError> {
        self.0
            .unbounded_send(packet.into())
            .map_err(|err| err.into_send_error())
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

pub type MixForwardingReceiver = mpsc::UnboundedReceiver<PacketToForward>;

pub struct PacketToForward {
    pub packet: MixPacket,
    pub forward_delay_target: Option<Instant>,
}

impl From<MixPacket> for PacketToForward {
    fn from(packet: MixPacket) -> Self {
        PacketToForward::new_no_delay(packet)
    }
}

impl From<(MixPacket, Option<Instant>)> for PacketToForward {
    fn from((packet, delay_until): (MixPacket, Option<Instant>)) -> Self {
        PacketToForward::new(packet, delay_until)
    }
}

impl From<(MixPacket, Instant)> for PacketToForward {
    fn from((packet, delay_until): (MixPacket, Instant)) -> Self {
        PacketToForward::new(packet, Some(delay_until))
    }
}

impl PacketToForward {
    pub fn new(packet: MixPacket, forward_delay_target: Option<Instant>) -> Self {
        PacketToForward {
            packet,
            forward_delay_target,
        }
    }

    pub fn new_no_delay(packet: MixPacket) -> Self {
        Self::new(packet, None)
    }
}

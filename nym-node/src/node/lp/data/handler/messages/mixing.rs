// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lp_data::packet::frame::{LpFrameHeader, LpFrameKind, SphinxFrameAttributes};
use nym_node_metrics::mixnet::PacketKind;
use nym_sphinx_forwarding::packet::MixPacketFormattingError;

use crate::node::lp::data::handler::error::LpDataHandlerError;

/// Message types supported by nym-nodes with only mixnode role
#[derive(Debug, Clone, Copy)]
pub enum MixMessage {
    Sphinx(SphinxFrameAttributes),
}

impl TryFrom<LpFrameHeader> for MixMessage {
    type Error = LpDataHandlerError;

    fn try_from(value: LpFrameHeader) -> Result<Self, Self::Error> {
        match value.kind {
            LpFrameKind::SphinxPacket => {
                let attributes = SphinxFrameAttributes::try_from(value.frame_attributes)
                    .map_err(MixPacketFormattingError::InvalidKeyRotation)?;
                Ok(MixMessage::Sphinx(attributes))
            }
            _ => Err(LpDataHandlerError::UnsupportedLpFrameKind { typ: value.kind }),
        }
    }
}

impl From<MixMessage> for PacketKind {
    fn from(value: MixMessage) -> Self {
        match value {
            MixMessage::Sphinx(_) => PacketKind::LpSphinx,
        }
    }
}

impl From<MixMessage> for LpFrameHeader {
    fn from(value: MixMessage) -> Self {
        match value {
            MixMessage::Sphinx(msg) => LpFrameHeader::new(LpFrameKind::SphinxPacket, msg),
        }
    }
}

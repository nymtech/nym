// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lp_data::packet::frame::{
    ForwardSphinxFrameAttributes, LpFrameHeader, LpFrameKind, SphinxFrameAttributes,
};
use nym_node_metrics::mixnet::PacketKind;
use nym_sphinx_forwarding::packet::MixPacketFormattingError;

use crate::node::lp::data::handler::{error::LpDataHandlerError, messages::MixMessage};

/// Message types supported by nym-nodes with a gateway role.
#[derive(Debug, Clone, Copy)]
pub enum NymNodeMessage {
    Mix(MixMessage),
    ForwardSphinx(ForwardSphinxFrameAttributes),
}

impl NymNodeMessage {
    pub fn new_sphinx_mix_message(message: SphinxFrameAttributes) -> Self {
        Self::Mix(MixMessage::Sphinx(message))
    }
}

impl TryFrom<LpFrameHeader> for NymNodeMessage {
    type Error = LpDataHandlerError;

    fn try_from(value: LpFrameHeader) -> Result<Self, Self::Error> {
        match value.kind {
            LpFrameKind::SphinxPacket => Ok(NymNodeMessage::Mix(value.try_into()?)),
            LpFrameKind::ForwardSphinxPacket => {
                let attributes = ForwardSphinxFrameAttributes::try_from(value.frame_attributes)
                    .map_err(MixPacketFormattingError::InvalidKeyRotation)?;
                Ok(NymNodeMessage::ForwardSphinx(attributes))
            }
            _ => Err(LpDataHandlerError::UnsupportedLpFrameKind { typ: value.kind }),
        }
    }
}

impl From<NymNodeMessage> for PacketKind {
    fn from(value: NymNodeMessage) -> Self {
        match value {
            NymNodeMessage::Mix(msg) => msg.into(),
            NymNodeMessage::ForwardSphinx(_) => PacketKind::LpSphinx,
        }
    }
}

impl From<MixMessage> for NymNodeMessage {
    fn from(value: MixMessage) -> Self {
        NymNodeMessage::Mix(value)
    }
}

impl From<NymNodeMessage> for LpFrameHeader {
    fn from(value: NymNodeMessage) -> Self {
        match value {
            NymNodeMessage::Mix(msg) => msg.into(),
            NymNodeMessage::ForwardSphinx(msg) => {
                LpFrameHeader::new(LpFrameKind::ForwardSphinxPacket, msg)
            }
        }
    }
}

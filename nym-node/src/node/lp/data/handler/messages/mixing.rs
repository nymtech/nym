// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lp_data::packet::frame::{LpFrameAttributes, LpFrameHeader, LpFrameKind};
use nym_node_metrics::mixnet::PacketKind;
use nym_sphinx_forwarding::packet::MixPacketFormattingError;
use nym_sphinx_params::SphinxKeyRotation;

use crate::node::lp::data::handler::error::LpDataHandlerError;

/// Message types supported by nym-nodes with only mixnode role
#[derive(Debug, Clone, Copy)]
pub enum MixMessage {
    Sphinx(SphinxMixMessage),
}

impl TryFrom<LpFrameHeader> for MixMessage {
    type Error = LpDataHandlerError;

    fn try_from(value: LpFrameHeader) -> Result<Self, Self::Error> {
        match value.kind {
            LpFrameKind::SphinxPacket => Ok(MixMessage::Sphinx(value.frame_attributes.try_into()?)),
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

#[derive(Debug, Clone, Copy)]
pub struct SphinxMixMessage {
    pub key_rotation: SphinxKeyRotation,
}

impl TryFrom<LpFrameAttributes> for SphinxMixMessage {
    type Error = LpDataHandlerError;

    fn try_from(value: LpFrameAttributes) -> Result<Self, Self::Error> {
        let key_rotation = value[0]
            .try_into()
            .map_err(MixPacketFormattingError::InvalidKeyRotation)?;
        Ok(SphinxMixMessage { key_rotation })
    }
}

impl From<SphinxMixMessage> for LpFrameAttributes {
    fn from(value: SphinxMixMessage) -> Self {
        let mut attrs = [0; 14];
        attrs[0] = value.key_rotation as u8;
        attrs
    }
}

// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lp_data::packet::frame::{LpFrameAttributes, LpFrameHeader, LpFrameKind};
use nym_node_metrics::mixnet::PacketKind;
use nym_sphinx_forwarding::packet::MixPacketFormattingError;
use nym_sphinx_params::SphinxKeyRotation;
use nym_topology::NodeId;

use crate::node::lp::data::handler::{
    error::LpDataHandlerError,
    messages::{MixMessage, SphinxMixMessage},
};

/// Message types supported by nym-nodes with a gateway role.
#[derive(Debug, Clone, Copy)]
pub enum NymNodeMessage {
    Mix(MixMessage),
    ForwardSphinx(ForwardSphinxMessage),
}

impl NymNodeMessage {
    pub fn new_sphinx_mix_message(message: SphinxMixMessage) -> Self {
        Self::Mix(MixMessage::Sphinx(message))
    }
}

impl TryFrom<LpFrameHeader> for NymNodeMessage {
    type Error = LpDataHandlerError;

    fn try_from(value: LpFrameHeader) -> Result<Self, Self::Error> {
        match value.kind {
            LpFrameKind::SphinxPacket => Ok(NymNodeMessage::Mix(value.try_into()?)),
            LpFrameKind::ForwardSphinxPacket => Ok(NymNodeMessage::ForwardSphinx(
                value.frame_attributes.try_into()?,
            )),
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

#[derive(Debug, Clone, Copy)]
pub struct ForwardSphinxMessage {
    pub key_rotation: SphinxKeyRotation,
    pub next_hop: NodeId,
}

impl TryFrom<LpFrameAttributes> for ForwardSphinxMessage {
    type Error = LpDataHandlerError;

    fn try_from(value: LpFrameAttributes) -> Result<Self, Self::Error> {
        let key_rotation = value[0]
            .try_into()
            .map_err(MixPacketFormattingError::InvalidKeyRotation)?;
        // SAFETY : slice to array conversion with correct size
        #[allow(clippy::unwrap_used)]
        let next_hop = NodeId::from_be_bytes(value[1..5].try_into().unwrap());
        Ok(ForwardSphinxMessage {
            key_rotation,
            next_hop,
        })
    }
}

impl From<ForwardSphinxMessage> for LpFrameAttributes {
    fn from(value: ForwardSphinxMessage) -> Self {
        let mut attrs = [0; 14];
        attrs[0] = value.key_rotation as u8;
        attrs[1..5].copy_from_slice(&value.next_hop.to_be_bytes());
        attrs
    }
}

impl From<ForwardSphinxMessage> for SphinxMixMessage {
    fn from(value: ForwardSphinxMessage) -> Self {
        SphinxMixMessage {
            key_rotation: value.key_rotation,
        }
    }
}

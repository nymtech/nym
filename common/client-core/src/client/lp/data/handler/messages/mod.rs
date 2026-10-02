// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lp_data::packet::frame::{LpFrameAttributes, LpFrameHeader, LpFrameKind};
use nym_sphinx::forwarding::packet::MixPacketFormattingError;
use nym_sphinx::params::SphinxKeyRotation;

use crate::client::lp::data::handler::error::LpDataHandlerError;

/// Message types supported by clients
#[derive(Debug, Clone, Copy)]
pub enum ClientMessage {
    Sphinx(SphinxMessage),
}

impl ClientMessage {
    pub fn from_frame_header(header: LpFrameHeader) -> Result<Self, LpDataHandlerError> {
        match header.kind {
            LpFrameKind::SphinxPacket => {
                Ok(ClientMessage::Sphinx(header.frame_attributes.try_into()?))
            }
            _ => Err(LpDataHandlerError::UnsupportedLpFrameKind { typ: header.kind }),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SphinxMessage {
    pub key_rotation: SphinxKeyRotation,
}

impl TryFrom<LpFrameAttributes> for SphinxMessage {
    type Error = LpDataHandlerError;

    fn try_from(value: LpFrameAttributes) -> Result<Self, Self::Error> {
        let key_rotation = value[0]
            .try_into()
            .map_err(MixPacketFormattingError::InvalidKeyRotation)?;
        Ok(SphinxMessage { key_rotation })
    }
}

impl From<SphinxMessage> for LpFrameAttributes {
    fn from(value: SphinxMessage) -> Self {
        let mut attrs = [0; 14];
        attrs[0] = value.key_rotation as u8;
        attrs
    }
}

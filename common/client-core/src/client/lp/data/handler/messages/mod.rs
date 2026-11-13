// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lp_data::packet::frame::{LpFrameHeader, LpFrameKind, SphinxFrameAttributes};
use nym_sphinx::forwarding::packet::MixPacketFormattingError;

use crate::client::lp::data::handler::error::LpDataHandlerError;

/// Message types supported by clients
#[derive(Debug, Clone, Copy)]
pub enum ClientMessage {
    Sphinx(SphinxFrameAttributes),
}

impl ClientMessage {
    pub fn from_frame_header(header: LpFrameHeader) -> Result<Self, LpDataHandlerError> {
        match header.kind {
            LpFrameKind::SphinxPacket => {
                let attributes = SphinxFrameAttributes::try_from(header.frame_attributes)
                    .map_err(MixPacketFormattingError::InvalidKeyRotation)?;
                Ok(ClientMessage::Sphinx(attributes))
            }
            _ => Err(LpDataHandlerError::UnsupportedLpFrameKind { typ: header.kind }),
        }
    }
}

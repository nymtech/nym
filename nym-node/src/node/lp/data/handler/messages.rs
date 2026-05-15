// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lp_data::{fragmentation::fragment::FragmentMetadata, packet::frame::LpFrameKind};
use nym_sphinx_forwarding::packet::MixPacketFormattingError;
use nym_sphinx_params::SphinxKeyRotation;

use crate::node::lp::data::handler::error::LpDataHandlerError;

#[derive(Debug, Clone, Copy)]
pub enum MixMessage {
    Sphinx {
        key_rotation: SphinxKeyRotation,
        reserved: [u8; 3],
    },
    Outfox {
        key_rotation: SphinxKeyRotation,
        reserved: [u8; 3],
    },
}

impl TryFrom<FragmentMetadata> for MixMessage {
    type Error = LpDataHandlerError;

    fn try_from(value: FragmentMetadata) -> Result<Self, Self::Error> {
        let key_rotation = value.metadata()[0]
            .try_into()
            .map_err(MixPacketFormattingError::InvalidKeyRotation)?;
        // SAFETY : correct length casting
        #[allow(clippy::unwrap_used)]
        let reserved = value.metadata()[1..4].try_into().unwrap();
        match value.kind() {
            LpFrameKind::FragmentedOutfoxPacket => Ok(MixMessage::Outfox {
                key_rotation,
                reserved,
            }),
            LpFrameKind::FragmentedSphinxPacket => Ok(MixMessage::Sphinx {
                key_rotation,
                reserved,
            }),
            _ => Err(LpDataHandlerError::UnexpectedLpPayload { typ: value.kind() }),
        }
    }
}

impl From<MixMessage> for FragmentMetadata {
    fn from(value: MixMessage) -> Self {
        match value {
            MixMessage::Outfox {
                key_rotation,
                reserved,
            } => {
                let metadata = [key_rotation as u8, reserved[0], reserved[1], reserved[2]];
                (LpFrameKind::FragmentedOutfoxPacket, metadata).into()
            }
            MixMessage::Sphinx {
                key_rotation,
                reserved,
            } => {
                let metadata = [key_rotation as u8, reserved[0], reserved[1], reserved[2]];
                (LpFrameKind::FragmentedSphinxPacket, metadata).into()
            }
        }
    }
}

// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lp_data::packet::frame::LpFrameKind;

use crate::node::lp::error::LpHandlerError;

pub enum MixMessage {
    Sphinx,
    Outfox,
}

impl TryFrom<LpFrameKind> for MixMessage {
    type Error = LpHandlerError;

    fn try_from(value: LpFrameKind) -> Result<Self, Self::Error> {
        match value {
            LpFrameKind::FragmentedOutfoxPacket => Ok(MixMessage::Outfox),
            LpFrameKind::FragmentedSphinxPacket => Ok(MixMessage::Sphinx),
            _ => Err(LpHandlerError::UnexpectedLpPayload { typ: value }),
        }
    }
}

impl From<MixMessage> for LpFrameKind {
    fn from(value: MixMessage) -> Self {
        match value {
            MixMessage::Outfox => LpFrameKind::FragmentedOutfoxPacket,
            MixMessage::Sphinx => LpFrameKind::FragmentedSphinxPacket,
        }
    }
}

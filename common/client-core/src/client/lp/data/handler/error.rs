// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lp_data::packet::frame::LpFrameKind;
use nym_sphinx::SphinxError;
use nym_sphinx::addressing::nodes::NymNodeRoutingAddressError;
use nym_sphinx::forwarding::packet::MixPacketFormattingError;
use nym_sphinx::framing::processing::PacketProcessingError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LpDataHandlerError {
    #[error(transparent)]
    PacketFormattingError(#[from] MixPacketFormattingError),

    #[error(transparent)]
    PacketProcessingError(#[from] PacketProcessingError),

    #[error(transparent)]
    NymNodeRoutingAddressError(#[from] NymNodeRoutingAddressError),

    #[error("failed to process received sphinx packet: {0}")]
    SphinxProcessingError(#[from] SphinxError),

    #[error("received payload type of an unexpected type: {typ:?}")]
    UnexpectedLpPayload { typ: LpFrameKind },

    #[error("received an Lp Frame kind that we don't support: {typ:?}")]
    UnsupportedLpFrameKind { typ: LpFrameKind },

    #[error("unwrapped a packet into a forward hop packet. This is no longer supported")]
    ForwardHop,

    #[error("{0}")]
    Internal(String),

    #[error("{0}")]
    Other(String),
}

impl LpDataHandlerError {
    pub fn internal(message: impl Into<String>) -> Self {
        LpDataHandlerError::Internal(message.into())
    }

    pub fn other(message: impl Into<String>) -> Self {
        LpDataHandlerError::Other(message.into())
    }
}

// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sphinx_acknowledgements::surb_ack::SurbAckRecoveryError;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddressError;
use nym_sphinx_types::{NymPacketError, OutfoxError, SphinxError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MixProcessingError {
    #[error("failed to process received packet: {0}")]
    NymPacketProcessingError(#[from] NymPacketError),

    #[error("failed to process received sphinx packet: {0}")]
    SphinxProcessingError(#[from] SphinxError),

    #[error("the forward hop address was malformed: {0}")]
    InvalidForwardHopAddress(#[from] NymNodeRoutingAddressError),

    #[error("the final hop did not contain a SURB-Ack")]
    NoSurbAckInFinalHop,

    #[error("failed to recover the expected SURB-Ack packet: {0}")]
    MalformedSurbAck(#[from] SurbAckRecoveryError),

    #[error("the received packet was set to use the very old and very much deprecated 'VPN' mode")]
    ReceivedOldTypeVpnPacket,

    #[error("failed to process received outfox packet: {0}")]
    OutfoxProcessingError(#[from] OutfoxError),

    #[error("this packet was already processed, it's a replay")]
    ReplayedPacketDetected,
}

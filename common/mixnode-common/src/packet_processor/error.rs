// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_sphinx_acknowledgements::surb_ack::SurbAckRecoveryError;
use nym_sphinx_framing::processing::PacketProcessingError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MixProcessingError {
    #[error("failed to recover the expected SURB-Ack packet: {0}")]
    MalformedSurbAck(#[from] SurbAckRecoveryError),

    #[error("the received packet was set to use the very old and very much deprecated 'VPN' mode")]
    ReceivedOldTypeVpnPacket,

    #[error("failed to process received sphinx packet: {0}")]
    NymPacketProcessingError(#[from] PacketProcessingError),
}

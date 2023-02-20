// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use mixnode_common::packet_processor::error::MixProcessingError;
pub use mixnode_common::packet_processor::processor::MixProcessingResult;
use mixnode_common::packet_processor::processor::{ProcessedFinalHop, SphinxPacketProcessor};
use nym_crypto::asymmetric::encryption;
use nym_sphinx::framing::packet::FramedSphinxPacket;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GatewayProcessingError {
    #[error("failed to process received mix packet - {0}")]
    PacketProcessingError(#[from] MixProcessingError),

    #[error("received a forward hop mix packet")]
    ForwardHopReceivedError,
}

// PacketProcessor contains all data required to correctly unwrap and store sphinx packets
#[derive(Clone)]
pub struct PacketProcessor {
    inner_processor: SphinxPacketProcessor,
}

impl PacketProcessor {
    pub(crate) fn new(encryption_key: &encryption::PrivateKey) -> Self {
        PacketProcessor {
            inner_processor: SphinxPacketProcessor::new(encryption_key.into()),
        }
    }

    pub(crate) fn process_received(
        &self,
        received: FramedSphinxPacket,
    ) -> Result<ProcessedFinalHop, GatewayProcessingError> {
        match self.inner_processor.process_received(received)? {
            MixProcessingResult::ForwardHop(..) => {
                Err(GatewayProcessingError::ForwardHopReceivedError)
            }
            MixProcessingResult::FinalHop(processed_final) => Ok(processed_final),
        }
    }
}

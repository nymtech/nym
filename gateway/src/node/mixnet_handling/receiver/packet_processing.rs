// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crypto::asymmetric::encryption;
use mixnode_common::packet_processor::error::MixProcessingError;
pub use mixnode_common::packet_processor::processor::MixProcessingResult;
use mixnode_common::packet_processor::processor::{ProcessedFinalHop, SphinxPacketProcessor};
use nymsphinx::framing::packet::FramedSphinxPacket;

#[derive(Debug)]
pub enum GatewayProcessingError {
    PacketProcessingError(MixProcessingError),
    ForwardHopReceivedError,
}

impl From<MixProcessingError> for GatewayProcessingError {
    fn from(e: MixProcessingError) -> Self {
        use GatewayProcessingError::*;

        PacketProcessingError(e)
    }
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

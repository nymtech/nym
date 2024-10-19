// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_crypto::asymmetric::encryption;
use nym_mixnode_common::packet_processor::error::MixProcessingError;
use nym_mixnode_common::packet_processor::processor::SphinxPacketProcessor;
use nym_sphinx::framing::packet::FramedNymPacket;
use nym_sphinx::framing::processing::{
    process_framed_packet, MixProcessingResult, PacketProcessingError, ProcessedFinalHop,
};
use nym_sphinx::PrivateKey;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GatewayProcessingError {
    #[error("failed to process received mix packet - {0}")]
    PacketProcessing(#[from] MixProcessingError),

    #[error("received a forward hop mix packet")]
    ForwardHopReceived,

    #[error("failed to process received sphinx packet: {0}")]
    NymPacketProcessing(#[from] PacketProcessingError),
}

// PacketProcessor contains all data required to correctly unwrap and store sphinx packets
#[derive(Clone)]
pub struct PacketProcessor {
    inner_processor: SphinxPacketProcessor,
}

impl PacketProcessor {
    pub fn sphinx_key(&self) -> &PrivateKey {
        self.inner_processor.sphinx_key()
    }

    pub(crate) fn new(encryption_key: &encryption::PrivateKey) -> Self {
        PacketProcessor {
            inner_processor: SphinxPacketProcessor::new(encryption_key.into()),
        }
    }
}

pub(crate) fn process_packet(
    received: FramedNymPacket,
    sphinx_key: &nym_sphinx::PrivateKey,
) -> Result<ProcessedFinalHop, GatewayProcessingError> {
    match process_framed_packet(received, sphinx_key)? {
        MixProcessingResult::ForwardHop(..) => Err(GatewayProcessingError::ForwardHopReceived),
        MixProcessingResult::FinalHop(processed_final) => Ok(processed_final),
    }
}

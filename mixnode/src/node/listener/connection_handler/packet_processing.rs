// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::node_statistics;
use nym_crypto::asymmetric::encryption;
use nym_mixnode_common::packet_processor::error::MixProcessingError;
use nym_mixnode_common::packet_processor::processor::SphinxPacketProcessor;
use nym_sphinx::framing::packet::FramedNymPacket;
use nym_sphinx::framing::processing::{process_framed_packet, MixProcessingResult};

// PacketProcessor contains all data required to correctly unwrap and forward sphinx packets
#[derive(Clone)]
pub(crate) struct PacketProcessor {
    /// Responsible for performing unwrapping
    inner_processor: SphinxPacketProcessor,

    /// Responsible for updating metrics data
    node_stats_update_sender: node_statistics::UpdateSender,
}

impl PacketProcessor {
    pub(crate) fn new(
        encryption_key: &encryption::PrivateKey,
        node_stats_update_sender: node_statistics::UpdateSender,
    ) -> Self {
        PacketProcessor {
            inner_processor: SphinxPacketProcessor::new(encryption_key.into()),
            node_stats_update_sender,
        }
    }

    pub fn inner(&self) -> &SphinxPacketProcessor {
        &self.inner_processor
    }

    pub fn node_stats_update_sender(&self) -> &node_statistics::UpdateSender {
        &self.node_stats_update_sender
    }
}

pub fn process_received_packet(
    packet: FramedNymPacket,
    inner_processor: &SphinxPacketProcessor,
) -> Result<MixProcessingResult, MixProcessingError> {
    Ok(process_framed_packet(packet, inner_processor.sphinx_key())?)
}

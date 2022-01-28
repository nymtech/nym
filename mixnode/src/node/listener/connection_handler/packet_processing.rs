// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::statistics;
use crypto::asymmetric::encryption;
use mixnode_common::packet_processor::error::MixProcessingError;
pub use mixnode_common::packet_processor::processor::MixProcessingResult;
use mixnode_common::packet_processor::processor::SphinxPacketProcessor;
use nymsphinx::framing::packet::FramedSphinxPacket;

// PacketProcessor contains all data required to correctly unwrap and forward sphinx packets
#[derive(Clone)]
pub struct PacketProcessor {
    /// Responsible for performing unwrapping
    inner_processor: SphinxPacketProcessor,

    /// Responsible for updating metrics data
    node_stats_update_sender: statistics::PacketEventReporter,
}

impl PacketProcessor {
    pub(crate) fn new(
        encryption_key: &encryption::PrivateKey,
        node_stats_update_sender: statistics::PacketEventReporter,
    ) -> Self {
        PacketProcessor {
            inner_processor: SphinxPacketProcessor::new(encryption_key.into()),
            node_stats_update_sender,
        }
    }

    pub(crate) fn process_received(
        &self,
        received: FramedSphinxPacket,
    ) -> Result<MixProcessingResult, MixProcessingError> {
        self.node_stats_update_sender.report_received();
        self.inner_processor.process_received(received)
    }
}

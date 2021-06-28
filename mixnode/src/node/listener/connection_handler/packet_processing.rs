// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::node_statistics;
use crypto::asymmetric::encryption;
use mixnode_common::cached_packet_processor::error::MixProcessingError;
use mixnode_common::cached_packet_processor::processor::CachedPacketProcessor;
pub use mixnode_common::cached_packet_processor::processor::MixProcessingResult;
use nymsphinx::framing::packet::FramedSphinxPacket;
use tokio::time::Duration;

// PacketProcessor contains all data required to correctly unwrap and forward sphinx packets
pub struct PacketProcessor {
    /// Responsible for performing unwrapping
    inner_processor: CachedPacketProcessor,

    /// Responsible for updating metrics data
    node_stats_update_sender: node_statistics::UpdateSender,
}

impl PacketProcessor {
    pub(crate) fn new(
        encryption_key: &encryption::PrivateKey,
        node_stats_update_sender: node_statistics::UpdateSender,
        cache_entry_ttl: Duration,
    ) -> Self {
        PacketProcessor {
            inner_processor: CachedPacketProcessor::new(encryption_key.into(), cache_entry_ttl),
            node_stats_update_sender,
        }
    }

    pub(crate) fn clone_without_cache(&self) -> Self {
        PacketProcessor {
            inner_processor: self.inner_processor.clone_without_cache(),
            node_stats_update_sender: self.node_stats_update_sender.clone(),
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

// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::metrics;
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
    metrics_reporter: metrics::MetricsReporter,
}

impl PacketProcessor {
    pub(crate) fn new(
        encryption_key: &encryption::PrivateKey,
        metrics_reporter: metrics::MetricsReporter,
        cache_entry_ttl: Duration,
    ) -> Self {
        PacketProcessor {
            inner_processor: CachedPacketProcessor::new(encryption_key.into(), cache_entry_ttl),
            metrics_reporter,
        }
    }

    pub(crate) fn clone_without_cache(&self) -> Self {
        PacketProcessor {
            inner_processor: self.inner_processor.clone_without_cache(),
            metrics_reporter: self.metrics_reporter.clone(),
        }
    }

    pub(crate) fn process_received(
        &self,
        received: FramedSphinxPacket,
    ) -> Result<MixProcessingResult, MixProcessingError> {
        self.metrics_reporter.report_received();
        self.inner_processor.process_received(received)
    }
}

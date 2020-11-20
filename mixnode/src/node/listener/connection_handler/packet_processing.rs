// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::node::metrics;
use crypto::asymmetric::encryption;
use mixnode_common::cached_packet_processor::error::MixProcessingError;
use mixnode_common::cached_packet_processor::processor::CachedPacketProcessor;
pub use mixnode_common::cached_packet_processor::processor::MixProcessingResult;
use nymsphinx::addressing::nodes::NymNodeRoutingAddress;
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

    pub(crate) fn report_sent(&self, address: NymNodeRoutingAddress) {
        self.metrics_reporter.report_sent(address.to_string())
    }

    pub(crate) fn process_received(
        &self,
        received: FramedSphinxPacket,
    ) -> Result<MixProcessingResult, MixProcessingError> {
        // self.metrics_reporter.report_received(); -> TODO METRICS!
        self.inner_processor.process_received(received)
    }
}

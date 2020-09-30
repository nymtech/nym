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

use crypto::asymmetric::encryption;
use mixnode_common::cached_packet_processor::error::MixProcessingError;
pub use mixnode_common::cached_packet_processor::processor::MixProcessingResult;
use mixnode_common::cached_packet_processor::processor::{
    CachedPacketProcessor, ProcessedFinalHop,
};
use nymsphinx::framing::packet::FramedSphinxPacket;
use tokio::time::Duration;

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
pub struct PacketProcessor {
    inner_processor: CachedPacketProcessor,
}

impl PacketProcessor {
    pub(crate) fn new(encryption_key: &encryption::PrivateKey, cache_entry_ttl: Duration) -> Self {
        PacketProcessor {
            inner_processor: CachedPacketProcessor::new(encryption_key.into(), cache_entry_ttl),
        }
    }

    pub(crate) fn clone_without_key_cache(&self) -> Self {
        PacketProcessor {
            inner_processor: self.inner_processor.clone_without_cache(),
        }
    }

    pub(crate) async fn process_received(
        &self,
        received: FramedSphinxPacket,
    ) -> Result<ProcessedFinalHop, GatewayProcessingError> {
        match self.inner_processor.process_received(received).await? {
            MixProcessingResult::ForwardHop(..) => {
                Err(GatewayProcessingError::ForwardHopReceivedError)
            }
            MixProcessingResult::FinalHop(processed_final) => Ok(processed_final),
        }
    }
}

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

use crate::node::listener::connection_handler::CachedKeys;
use crate::node::metrics;
use crypto::asymmetric::encryption;
use log::*;
use nymsphinx::addressing::nodes::{NymNodeRoutingAddress, NymNodeRoutingAddressError};
use nymsphinx::header::{keys::RoutingKeys, SphinxHeader};
use nymsphinx::{
    Delay as SphinxDelay, Error as SphinxError, NodeAddressBytes, ProcessedPacket, SharedSecret,
    SphinxPacket,
};
use std::convert::TryFrom;
use std::sync::Arc;

#[derive(Debug)]
pub enum MixProcessingError {
    ReceivedFinalHopError,
    SphinxProcessingError(SphinxError),
    InvalidHopAddress,
}

pub enum MixProcessingResult {
    ForwardHop(NymNodeRoutingAddress, SphinxPacket),
    #[allow(dead_code)]
    LoopMessage,
}

impl From<SphinxError> for MixProcessingError {
    // for time being just have a single error instance for all possible results of SphinxError
    fn from(err: SphinxError) -> Self {
        use MixProcessingError::*;

        SphinxProcessingError(err)
    }
}

impl From<NymNodeRoutingAddressError> for MixProcessingError {
    fn from(_: NymNodeRoutingAddressError) -> Self {
        use MixProcessingError::*;

        InvalidHopAddress
    }
}

// PacketProcessor contains all data required to correctly unwrap and forward sphinx packets
#[derive(Clone)]
pub struct PacketProcessor {
    sphinx_key: Arc<nymsphinx::PrivateKey>,
    metrics_reporter: metrics::MetricsReporter,
}

impl PacketProcessor {
    pub(crate) fn new(
        encryption_key: &encryption::PrivateKey,
        metrics_reporter: metrics::MetricsReporter,
    ) -> Self {
        PacketProcessor {
            sphinx_key: Arc::new(encryption_key.into()),
            metrics_reporter,
        }
    }

    pub(crate) fn is_vpn_packet(&self, packet: &ProcessedPacket) -> bool {
        match packet {
            ProcessedPacket::ForwardHop(_, _, delay) => delay.to_nanos() == 0,
            ProcessedPacket::FinalHop(..) => false,
        }
    }

    pub(crate) fn report_sent(&self, addr: NymNodeRoutingAddress) {
        self.metrics_reporter.report_sent(addr.to_string())
    }

    async fn delay_packet(&self, delay: SphinxDelay) {
        // TODO: this should perhaps be replaced with a `DelayQueue`
        tokio::time::delay_for(delay.to_duration()).await;
    }

    async fn process_forward_hop(
        &self,
        packet: SphinxPacket,
        forward_address: NodeAddressBytes,
        delay: SphinxDelay,
    ) -> Result<MixProcessingResult, MixProcessingError> {
        let next_hop_address = NymNodeRoutingAddress::try_from(forward_address)?;

        // Delay packet for as long as required (don't call into the scheduler if it's 0!)
        if delay.to_nanos() != 0 {
            self.delay_packet(delay).await;
        }

        Ok(MixProcessingResult::ForwardHop(next_hop_address, packet))
    }

    pub(crate) fn recompute_routing_keys(&self, initial_secret: &SharedSecret) -> RoutingKeys {
        SphinxHeader::compute_routing_keys(initial_secret, &self.sphinx_key)
    }

    pub(crate) fn perform_initial_processing(
        &self,
        packet: SphinxPacket,
    ) -> Result<ProcessedPacket, MixProcessingError> {
        self.metrics_reporter.report_received();

        packet.process(&self.sphinx_key).map_err(|err| {
            warn!("Failed to unwrap Sphinx packet: {:?}", err);
            MixProcessingError::SphinxProcessingError(err)
        })
    }

    pub(crate) fn perform_initial_processing_with_cached_keys(
        &self,
        packet: SphinxPacket,
        keys: &CachedKeys,
    ) -> Result<ProcessedPacket, MixProcessingError> {
        self.metrics_reporter.report_received();

        packet
            .process_with_derived_keys(&keys.0, &keys.1)
            .map_err(|err| {
                warn!("Failed to unwrap Sphinx packet: {:?}", err);
                MixProcessingError::SphinxProcessingError(err)
            })
    }

    pub(crate) async fn perform_final_processing(
        &self,
        packet: ProcessedPacket,
    ) -> Result<MixProcessingResult, MixProcessingError> {
        match packet {
            ProcessedPacket::ForwardHop(packet, address, delay) => {
                self.process_forward_hop(packet, address, delay).await
            }
            ProcessedPacket::FinalHop(..) => {
                warn!("Received a loop cover message that we haven't implemented yet!");
                Err(MixProcessingError::ReceivedFinalHopError)
            }
        }
    }
}

// TODO: the test that definitely needs to be written is as follows:
// we are stuck trying to write to mix A, can we still forward just fine to mix B?

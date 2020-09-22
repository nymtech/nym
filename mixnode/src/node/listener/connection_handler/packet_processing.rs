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
use dashmap::DashMap;
use log::*;
use mixnet_client::forwarder::ForwardedPacket;
use nymsphinx::addressing::nodes::{NymNodeRoutingAddress, NymNodeRoutingAddressError};
use nymsphinx::framing::packet::FramedSphinxPacket;
use nymsphinx::header::{keys::RoutingKeys, SphinxHeader};
use nymsphinx::params::PacketMode;
use nymsphinx::{
    Delay as SphinxDelay, Error as SphinxError, NodeAddressBytes, ProcessedPacket, SharedSecret,
    SphinxPacket,
};
use std::convert::TryFrom;
use std::sync::Arc;

pub(crate) type CachedKeys = (Option<SharedSecret>, RoutingKeys);

#[derive(Debug)]
pub enum MixProcessingError {
    ReceivedFinalHopError,
    SphinxProcessingError(SphinxError),
    InvalidHopAddress,
}

pub enum MixProcessingResult {
    ForwardHop(ForwardedPacket),
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
pub struct PacketProcessor {
    sphinx_key: Arc<nymsphinx::PrivateKey>,
    metrics_reporter: metrics::MetricsReporter,

    // TODO: method for cache invalidation so that we wouldn't keep all keys for all eternity
    // we could use our friend DelayQueue. One of tokio's examples is literally using it for
    // cache invalidation: https://docs.rs/tokio/0.2.22/tokio/time/struct.DelayQueue.html
    vpn_key_cache: DashMap<SharedSecret, CachedKeys>,
}

impl PacketProcessor {
    pub(crate) fn new(
        encryption_key: &encryption::PrivateKey,
        metrics_reporter: metrics::MetricsReporter,
    ) -> Self {
        PacketProcessor {
            sphinx_key: Arc::new(encryption_key.into()),
            metrics_reporter,
            vpn_key_cache: DashMap::new(),
        }
    }

    pub(crate) fn clone_without_cache(&self) -> Self {
        PacketProcessor {
            sphinx_key: self.sphinx_key.clone(),
            metrics_reporter: self.metrics_reporter.clone(),
            vpn_key_cache: DashMap::new(),
        }
    }

    pub(crate) fn report_sent(&self, address: NymNodeRoutingAddress) {
        self.metrics_reporter.report_sent(address.to_string())
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
        packet_mode: PacketMode,
    ) -> Result<MixProcessingResult, MixProcessingError> {
        let next_hop_address = NymNodeRoutingAddress::try_from(forward_address)?;

        if !packet_mode.is_vpn() {
            self.delay_packet(delay).await;
        }

        let forwarded_packet = ForwardedPacket::new(next_hop_address, packet, packet_mode);
        Ok(MixProcessingResult::ForwardHop(forwarded_packet))
    }

    pub(crate) fn recompute_routing_keys(&self, initial_secret: &SharedSecret) -> RoutingKeys {
        SphinxHeader::compute_routing_keys(initial_secret, &self.sphinx_key)
    }

    fn perform_initial_packet_processing(
        &self,
        packet: SphinxPacket,
    ) -> Result<ProcessedPacket, MixProcessingError> {
        packet.process(&self.sphinx_key).map_err(|err| {
            warn!("Failed to unwrap Sphinx packet: {:?}", err);
            MixProcessingError::SphinxProcessingError(err)
        })
    }

    fn perform_initial_packet_processing_with_cached_keys(
        &self,
        packet: SphinxPacket,
        keys: &CachedKeys,
    ) -> Result<ProcessedPacket, MixProcessingError> {
        packet
            .process_with_derived_keys(&keys.0, &keys.1)
            .map_err(|err| {
                warn!("Failed to unwrap Sphinx packet: {:?}", err);
                MixProcessingError::SphinxProcessingError(err)
            })
    }

    fn cache_keys(&self, initial_secret: SharedSecret, processed_packet: &ProcessedPacket) {
        let new_shared_secret = processed_packet.shared_secret();
        let routing_keys = self.recompute_routing_keys(&initial_secret);
        if self
            .vpn_key_cache
            .insert(initial_secret, (new_shared_secret, routing_keys))
            .is_some()
        {
            warn!("We seem to have some weird replay issue - we already had cached keys for this packet!")
        }
    }

    fn pre_process_received(
        &self,
        received: FramedSphinxPacket,
    ) -> Result<ProcessedPacket, MixProcessingError> {
        let packet_mode = received.packet_mode();
        let sphinx_packet = received.into_inner();
        let initial_secret = sphinx_packet.shared_secret();

        // try to use pre-computed keys only for the vpn-packets
        if packet_mode.is_vpn() {
            if let Some(cached_keys) = self.vpn_key_cache.get(&initial_secret) {
                return self.perform_initial_packet_processing_with_cached_keys(
                    sphinx_packet,
                    cached_keys.value(),
                );
            }
        }

        let processing_result = self.perform_initial_packet_processing(sphinx_packet);
        // quicker exit because this will be the most common case
        if !packet_mode.is_vpn() {
            return processing_result;
        }

        if let Ok(processed_packet) = processing_result.as_ref() {
            // if we managed to process packet we saw for the first time AND it's a vpn packet
            // cache the keys
            self.cache_keys(initial_secret, processed_packet);
        }
        processing_result
    }

    async fn perform_final_processing(
        &self,
        packet: ProcessedPacket,
        packet_mode: PacketMode,
    ) -> Result<MixProcessingResult, MixProcessingError> {
        match packet {
            ProcessedPacket::ForwardHop(packet, address, delay) => {
                self.process_forward_hop(packet, address, delay, packet_mode)
                    .await
            }
            ProcessedPacket::FinalHop(..) => {
                warn!("Received a loop cover message that we haven't implemented yet!");
                Err(MixProcessingError::ReceivedFinalHopError)
            }
        }
    }

    pub(crate) async fn process_received(
        &self,
        received: FramedSphinxPacket,
    ) -> Result<MixProcessingResult, MixProcessingError> {
        self.metrics_reporter.report_received();
        let packet_mode = received.packet_mode();

        // unwrap the sphinx packet and if possible and appropriate, cache keys
        let processed_packet = self.pre_process_received(received)?;

        // for non-vpn packets delay for required time
        self.perform_final_processing(processed_packet, packet_mode)
            .await
    }
}

// TODO: the test that definitely needs to be written is as follows:
// we are stuck trying to write to mix A, can we still forward just fine to mix B?

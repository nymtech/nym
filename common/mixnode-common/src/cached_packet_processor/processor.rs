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

use crate::cached_packet_processor::cache::KeyCache;
use crate::cached_packet_processor::error::MixProcessingError;
use log::*;
use nymsphinx_acknowledgements::surb_ack::SurbAck;
use nymsphinx_addressing::nodes::NymNodeRoutingAddress;
use nymsphinx_forwarding::packet::MixPacket;
use nymsphinx_framing::packet::FramedSphinxPacket;
use nymsphinx_params::{PacketMode, PacketSize};
use nymsphinx_types::header::keys::RoutingKeys;
use nymsphinx_types::{
    Delay as SphinxDelay, DestinationAddressBytes, NodeAddressBytes, Payload, PrivateKey,
    ProcessedPacket, SharedSecret, SphinxHeader, SphinxPacket,
};
use std::convert::TryFrom;
use std::sync::Arc;
use tokio::time::Duration;

type ForwardAck = MixPacket;
type CachedKeys = (Option<SharedSecret>, RoutingKeys);

pub struct ProcessedFinalHop {
    pub destination: DestinationAddressBytes,
    pub forward_ack: Option<ForwardAck>,
    pub message: Vec<u8>,
}

pub enum MixProcessingResult {
    /// Contains unwrapped data that should first get delayed before being sent to next hop.
    ForwardHop(MixPacket, Option<SphinxDelay>),

    /// Contains all data extracted out of the final hop packet that could be forwarded to the destination.
    FinalHop(ProcessedFinalHop),
}

pub struct CachedPacketProcessor {
    /// Private sphinx key of this node required to unwrap received sphinx packet.
    sphinx_key: Arc<PrivateKey>,

    /// Key cache containing derived shared keys for packets using `vpn_mode`.
    // Note: as discovered this is potentially unsafe as security of Lioness depends on keys never being reused.
    // So perhaps it should get completely disabled for time being?
    vpn_key_cache: KeyCache,
}

impl CachedPacketProcessor {
    /// Creates new instance of `CachedPacketProcessor`
    pub fn new(sphinx_key: PrivateKey, cache_entry_ttl: Duration) -> Self {
        CachedPacketProcessor {
            sphinx_key: Arc::new(sphinx_key),
            vpn_key_cache: KeyCache::new(cache_entry_ttl),
        }
    }

    /// Clones `self` without the `vpn_key_cache`.
    pub fn clone_without_cache(&self) -> Self {
        CachedPacketProcessor {
            sphinx_key: self.sphinx_key.clone(),
            vpn_key_cache: KeyCache::new(self.vpn_key_cache.cache_entry_ttl()),
        }
    }

    /// Recomputes routing keys for the given initial secret.
    fn recompute_routing_keys(&self, initial_secret: &SharedSecret) -> RoutingKeys {
        SphinxHeader::compute_routing_keys(initial_secret, &self.sphinx_key)
    }

    /// Performs a fresh sphinx unwrapping using no cache.
    fn perform_initial_sphinx_packet_processing(
        &self,
        packet: SphinxPacket,
    ) -> Result<ProcessedPacket, MixProcessingError> {
        packet.process(&self.sphinx_key).map_err(|err| {
            debug!("Failed to unwrap Sphinx packet: {:?}", err);
            MixProcessingError::SphinxProcessingError(err)
        })
    }

    /// Unwraps sphinx packet using already cached keys.
    fn perform_initial_sphinx_packet_processing_with_cached_keys(
        &self,
        packet: SphinxPacket,
        keys: &CachedKeys,
    ) -> Result<ProcessedPacket, MixProcessingError> {
        packet
            .process_with_derived_keys(&keys.0, &keys.1)
            .map_err(|err| {
                debug!("Failed to unwrap Sphinx packet: {:?}", err);
                MixProcessingError::SphinxProcessingError(err)
            })
    }

    /// Stores the keys corresponding to the packet that was just processed.
    fn cache_keys(&self, initial_secret: SharedSecret, processed_packet: &ProcessedPacket) {
        let new_shared_secret = processed_packet.shared_secret();
        let routing_keys = self.recompute_routing_keys(&initial_secret);
        if self
            .vpn_key_cache
            .insert(initial_secret, (new_shared_secret, routing_keys))
        {
            debug!("Other thread has already cached keys for this secret!")
        }
    }

    /// Takes the received framed packet and tries to unwrap it from the sphinx encryption.
    /// For any vpn packets it will try to re-use cached keys and if none are available,
    /// after first processing, the keys are going to get cached.
    fn perform_initial_unwrapping(
        &self,
        received: FramedSphinxPacket,
    ) -> Result<ProcessedPacket, MixProcessingError> {
        let packet_mode = received.packet_mode();
        let sphinx_packet = received.into_inner();
        let initial_secret = sphinx_packet.shared_secret();

        // try to use pre-computed keys only for the vpn-packets
        if packet_mode.is_vpn() {
            if let Some(cached_keys) = self.vpn_key_cache.get(&initial_secret) {
                return self.perform_initial_sphinx_packet_processing_with_cached_keys(
                    sphinx_packet,
                    cached_keys.value(),
                );
            }
        }

        let processing_result = self.perform_initial_sphinx_packet_processing(sphinx_packet);
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

    /// Processed received forward hop packet - tries to extract next hop address, sets delay,
    /// if it was not a vpn packet and packs all the data in a way that can be easily sent
    /// to the next hop.
    fn process_forward_hop(
        &self,
        packet: SphinxPacket,
        forward_address: NodeAddressBytes,
        delay: SphinxDelay,
        packet_mode: PacketMode,
    ) -> Result<MixProcessingResult, MixProcessingError> {
        let next_hop_address = NymNodeRoutingAddress::try_from(forward_address)?;

        // if the packet is set to vpn mode, ignore whatever might have been set as delay
        let delay = if packet_mode.is_vpn() {
            None
        } else {
            Some(delay)
        };

        let mix_packet = MixPacket::new(next_hop_address, packet, packet_mode);
        Ok(MixProcessingResult::ForwardHop(mix_packet, delay))
    }

    /// Split data extracted from the final hop sphinx packet into a SURBAck and message
    /// that should get delivered to a client.
    fn split_hop_data_into_ack_and_message(
        &self,
        mut extracted_data: Vec<u8>,
    ) -> Result<(Vec<u8>, Vec<u8>), MixProcessingError> {
        // in theory it's impossible for this to fail since it managed to go into correct `match`
        // branch at the caller
        if extracted_data.len() < SurbAck::len() {
            return Err(MixProcessingError::NoSurbAckInFinalHop);
        }

        let message = extracted_data.split_off(SurbAck::len());
        let ack_data = extracted_data;
        Ok((ack_data, message))
    }

    /// Tries to extract a SURBAck that could be sent back into the mix network and message
    /// that should get delivered to a client from received Sphinx packet.
    fn split_into_ack_and_message(
        &self,
        data: Vec<u8>,
        packet_size: PacketSize,
        packet_mode: PacketMode,
    ) -> Result<(Option<MixPacket>, Vec<u8>), MixProcessingError> {
        match packet_size {
            PacketSize::AckPacket => {
                trace!("received an ack packet!");
                Ok((None, data))
            }
            PacketSize::RegularPacket | PacketSize::ExtendedPacket => {
                trace!("received a normal packet!");
                let (ack_data, message) = self.split_hop_data_into_ack_and_message(data)?;
                let (ack_first_hop, ack_packet) = SurbAck::try_recover_first_hop_packet(&ack_data)?;
                let forward_ack = MixPacket::new(ack_first_hop, ack_packet, packet_mode);
                Ok((Some(forward_ack), message))
            }
        }
    }

    /// Processed received final hop packet - tries to extract SURBAck out of it (assuming the
    /// packet itself is not an ACK) and splits it from the message that should get delivered
    /// to the destination.
    fn process_final_hop(
        &self,
        destination: DestinationAddressBytes,
        payload: Payload,
        packet_size: PacketSize,
        packet_mode: PacketMode,
    ) -> Result<MixProcessingResult, MixProcessingError> {
        let packet_message = payload.recover_plaintext()?;

        let (forward_ack, message) =
            self.split_into_ack_and_message(packet_message, packet_size, packet_mode)?;

        Ok(MixProcessingResult::FinalHop(ProcessedFinalHop {
            destination,
            forward_ack,
            message,
        }))
    }

    /// Performs final processing for the unwrapped packet based on whether it was a forward hop
    /// or a final hop.
    fn perform_final_processing(
        &self,
        packet: ProcessedPacket,
        packet_size: PacketSize,
        packet_mode: PacketMode,
    ) -> Result<MixProcessingResult, MixProcessingError> {
        match packet {
            ProcessedPacket::ForwardHop(packet, address, delay) => {
                self.process_forward_hop(packet, address, delay, packet_mode)
            }
            // right now there's no use for the surb_id included in the header - probably it should get removed from the
            // sphinx all together?
            ProcessedPacket::FinalHop(destination, _, payload) => {
                self.process_final_hop(destination, payload, packet_size, packet_mode)
            }
        }
    }

    pub fn process_received(
        &self,
        received: FramedSphinxPacket,
    ) -> Result<MixProcessingResult, MixProcessingError> {
        // explicit packet size will help to correctly parse final hop
        let packet_size = received.packet_size();
        let packet_mode = received.packet_mode();

        // unwrap the sphinx packet and if possible and appropriate, cache keys
        let processed_packet = self.perform_initial_unwrapping(received)?;

        // for forward packets, extract next hop and set delay (but do NOT delay here)
        // for final packets, extract SURBAck
        self.perform_final_processing(processed_packet, packet_size, packet_mode)
    }
}

// TODO: what more could we realistically test here?
#[cfg(test)]
mod tests {
    use super::*;
    use nymsphinx_types::builder::SphinxPacketBuilder;
    use nymsphinx_types::crypto::keygen;
    use nymsphinx_types::{
        Destination, Node, PublicKey, DESTINATION_ADDRESS_LENGTH, IDENTIFIER_LENGTH,
    };
    use std::convert::TryInto;
    use std::net::SocketAddr;

    fn fixture() -> CachedPacketProcessor {
        let local_keys = keygen();
        CachedPacketProcessor::new(local_keys.0, Duration::from_secs(30))
    }

    fn make_valid_final_sphinx_packet(size: PacketSize, public_key: PublicKey) -> SphinxPacket {
        let routing_address: NymNodeRoutingAddress =
            NymNodeRoutingAddress::from("127.0.0.1:1789".parse::<SocketAddr>().unwrap());

        let node = Node::new(routing_address.try_into().unwrap(), public_key);

        let destination = Destination::new(
            DestinationAddressBytes::from_bytes([3u8; DESTINATION_ADDRESS_LENGTH]),
            [4u8; IDENTIFIER_LENGTH],
        );

        // required until https://github.com/nymtech/sphinx/issues/71 is fixed
        let dummy_delay = SphinxDelay::new_from_nanos(42);

        SphinxPacketBuilder::new()
            .with_payload_size(size.payload_size())
            .build_packet(b"foomp".to_vec(), &[node], &destination, &[dummy_delay])
            .unwrap()
    }

    fn make_valid_forward_sphinx_packet(size: PacketSize, public_key: PublicKey) -> SphinxPacket {
        let routing_address: NymNodeRoutingAddress =
            NymNodeRoutingAddress::from("127.0.0.1:1789".parse::<SocketAddr>().unwrap());

        let some_node_key = keygen();
        let route = [
            Node::new(routing_address.try_into().unwrap(), public_key),
            Node::new(routing_address.try_into().unwrap(), some_node_key.1),
        ];

        let destination = Destination::new(
            DestinationAddressBytes::from_bytes([3u8; DESTINATION_ADDRESS_LENGTH]),
            [4u8; IDENTIFIER_LENGTH],
        );

        let delays = [
            SphinxDelay::new_from_nanos(42),
            SphinxDelay::new_from_nanos(42),
        ];

        SphinxPacketBuilder::new()
            .with_payload_size(size.payload_size())
            .build_packet(b"foomp".to_vec(), &route, &destination, &delays)
            .unwrap()
    }

    #[tokio::test]
    async fn recomputing_routing_keys_derives_correct_set_of_keys() {
        let processor = fixture();
        let (_, initial_secret) = keygen();
        assert_eq!(
            processor.recompute_routing_keys(&initial_secret),
            SphinxHeader::compute_routing_keys(&initial_secret, &processor.sphinx_key)
        )
    }

    #[tokio::test]
    async fn caching_keys_updates_local_state_for_final_hop() {
        let local_keys = keygen();
        let processor = CachedPacketProcessor::new(local_keys.0, Duration::from_secs(30));
        assert!(processor.vpn_key_cache.is_empty());

        let final_hop = make_valid_final_sphinx_packet(Default::default(), local_keys.1);
        let initial_secret = final_hop.shared_secret();
        let processed = final_hop.process(&processor.sphinx_key).unwrap();

        processor.cache_keys(initial_secret, &processed);
        let cache_entry = processor.vpn_key_cache.get(&initial_secret).unwrap();

        let (cached_secret, cached_routing_keys) = cache_entry.value();

        assert!(cached_secret.is_none());
        let recomputed_keys = processor.recompute_routing_keys(&initial_secret);
        // if one key matches then all keys must match (or there is a serious bug inside sphinx)
        assert_eq!(
            cached_routing_keys.stream_cipher_key,
            recomputed_keys.stream_cipher_key
        );
    }

    #[tokio::test]
    async fn caching_keys_updates_local_state_for_forward_hop() {
        let local_keys = keygen();
        let processor = CachedPacketProcessor::new(local_keys.0, Duration::from_secs(30));
        assert!(processor.vpn_key_cache.is_empty());

        let forward_hop = make_valid_forward_sphinx_packet(Default::default(), local_keys.1);
        let initial_secret = forward_hop.shared_secret();
        let processed = forward_hop.process(&processor.sphinx_key).unwrap();

        processor.cache_keys(initial_secret, &processed);
        let cache_entry = processor.vpn_key_cache.get(&initial_secret).unwrap();

        let (cached_secret, cached_routing_keys) = cache_entry.value();

        assert_eq!(
            cached_secret.as_ref().unwrap(),
            processed.shared_secret().as_ref().unwrap()
        );
        let recomputed_keys = processor.recompute_routing_keys(&initial_secret);
        // if one key matches then all keys must match (or there is a serious bug inside sphinx)
        assert_eq!(
            cached_routing_keys.stream_cipher_key,
            recomputed_keys.stream_cipher_key
        );
    }

    #[tokio::test]
    async fn performing_initial_unwrapping_caches_keys_if_vpnmode_used_for_final_hop() {
        let local_keys = keygen();
        let processor = CachedPacketProcessor::new(local_keys.0, Duration::from_secs(30));
        assert!(processor.vpn_key_cache.is_empty());

        let final_hop = make_valid_final_sphinx_packet(Default::default(), local_keys.1);
        let framed = FramedSphinxPacket::new(final_hop, PacketMode::Vpn);

        processor.perform_initial_unwrapping(framed).unwrap();
        assert_eq!(processor.vpn_key_cache.len(), 1);
    }

    #[tokio::test]
    async fn performing_initial_unwrapping_caches_keys_if_vpnmode_used_for_forward_hop() {
        let local_keys = keygen();
        let processor = CachedPacketProcessor::new(local_keys.0, Duration::from_secs(30));
        assert!(processor.vpn_key_cache.is_empty());

        let forward_hop = make_valid_forward_sphinx_packet(Default::default(), local_keys.1);
        let framed = FramedSphinxPacket::new(forward_hop, PacketMode::Vpn);

        processor.perform_initial_unwrapping(framed).unwrap();
        assert_eq!(processor.vpn_key_cache.len(), 1);
    }

    #[tokio::test]
    async fn performing_initial_unwrapping_does_no_caching_for_mix_mode_for_final_hop() {
        let local_keys = keygen();
        let processor = CachedPacketProcessor::new(local_keys.0, Duration::from_secs(30));
        assert!(processor.vpn_key_cache.is_empty());

        let final_hop = make_valid_final_sphinx_packet(Default::default(), local_keys.1);
        let framed = FramedSphinxPacket::new(final_hop, PacketMode::Mix);

        processor.perform_initial_unwrapping(framed).unwrap();
        assert!(processor.vpn_key_cache.is_empty());
    }

    #[tokio::test]
    async fn performing_initial_unwrapping_does_no_caching_for_mix_mode_for_forward_hop() {
        let local_keys = keygen();
        let processor = CachedPacketProcessor::new(local_keys.0, Duration::from_secs(30));
        assert!(processor.vpn_key_cache.is_empty());

        let forward_hop = make_valid_forward_sphinx_packet(Default::default(), local_keys.1);
        let framed = FramedSphinxPacket::new(forward_hop, PacketMode::Mix);

        processor.perform_initial_unwrapping(framed).unwrap();
        assert!(processor.vpn_key_cache.is_empty());
    }

    #[tokio::test]
    async fn splitting_hop_data_works_for_sufficiently_long_payload() {
        let processor = fixture();

        let short_data = vec![42u8];
        assert!(processor
            .split_hop_data_into_ack_and_message(short_data)
            .is_err());

        let sufficient_data = vec![42u8; SurbAck::len()];
        let (ack, data) = processor
            .split_hop_data_into_ack_and_message(sufficient_data.clone())
            .unwrap();
        assert_eq!(sufficient_data, ack);
        assert!(data.is_empty());

        let long_data = vec![42u8; SurbAck::len() * 5];
        let (ack, data) = processor
            .split_hop_data_into_ack_and_message(long_data)
            .unwrap();
        assert_eq!(ack.len(), SurbAck::len());
        assert_eq!(data.len(), SurbAck::len() * 4)
    }

    #[tokio::test]
    async fn splitting_into_ack_and_message_returns_whole_data_for_ack() {
        let processor = fixture();

        let data = vec![42u8; SurbAck::len() + 10];
        let (ack, message) = processor
            .split_into_ack_and_message(data.clone(), PacketSize::AckPacket, Default::default())
            .unwrap();
        assert!(ack.is_none());
        assert_eq!(data, message)
    }
}

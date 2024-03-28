// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::packet_processor::error::MixProcessingError;
use log::*;
use nym_metrics::nanos;
use nym_sphinx_acknowledgements::surb_ack::SurbAck;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_forwarding::packet::MixPacket;
use nym_sphinx_framing::packet::FramedNymPacket;
use nym_sphinx_params::{PacketSize, PacketType};
use nym_sphinx_types::{
    Delay as SphinxDelay, DestinationAddressBytes, NodeAddressBytes, NymPacket, NymProcessedPacket,
    PrivateKey, ProcessedPacket,
};

use std::sync::Arc;

type ForwardAck = MixPacket;

#[derive(Debug)]
pub struct ProcessedFinalHop {
    pub destination: DestinationAddressBytes,
    pub forward_ack: Option<ForwardAck>,
    pub message: Vec<u8>,
}

#[derive(Debug)]
pub enum MixProcessingResult {
    /// Contains unwrapped data that should first get delayed before being sent to next hop.
    ForwardHop(MixPacket, Option<SphinxDelay>),

    /// Contains all data extracted out of the final hop packet that could be forwarded to the destination.
    FinalHop(ProcessedFinalHop),
}

#[derive(Clone)]
pub struct SphinxPacketProcessor {
    /// Private sphinx key of this node required to unwrap received sphinx packet.
    sphinx_key: Arc<PrivateKey>,
}

impl SphinxPacketProcessor {
    /// Creates new instance of `CachedPacketProcessor`
    pub fn new(sphinx_key: PrivateKey) -> Self {
        SphinxPacketProcessor {
            sphinx_key: Arc::new(sphinx_key),
        }
    }

    /// Performs a fresh sphinx unwrapping using no cache.
    fn perform_initial_packet_processing(
        &self,
        packet: NymPacket,
    ) -> Result<NymProcessedPacket, MixProcessingError> {
        nanos!("perform_initial_packet_processing", {
            packet.process(&self.sphinx_key).map_err(|err| {
                debug!("Failed to unwrap NymPacket packet: {err}");
                MixProcessingError::NymPacketProcessingError(err)
            })
        })
    }

    /// Takes the received framed packet and tries to unwrap it from the sphinx encryption.
    fn perform_initial_unwrapping(
        &self,
        received: FramedNymPacket,
    ) -> Result<NymProcessedPacket, MixProcessingError> {
        nanos!("perform_initial_unwrapping", {
            let packet = received.into_inner();
            self.perform_initial_packet_processing(packet)
        })
    }

    /// Processed received forward hop packet - tries to extract next hop address, sets delay
    /// and packs all the data in a way that can be easily sent to the next hop.
    fn process_forward_hop(
        &self,
        packet: NymPacket,
        forward_address: NodeAddressBytes,
        delay: SphinxDelay,
        packet_type: PacketType,
    ) -> Result<MixProcessingResult, MixProcessingError> {
        let next_hop_address = NymNodeRoutingAddress::try_from(forward_address)?;

        let mix_packet = MixPacket::new(next_hop_address, packet, packet_type);
        Ok(MixProcessingResult::ForwardHop(mix_packet, Some(delay)))
    }

    /// Split data extracted from the final hop sphinx packet into a SURBAck and message
    /// that should get delivered to a client.
    fn split_hop_data_into_ack_and_message(
        &self,
        mut extracted_data: Vec<u8>,
        packet_type: PacketType,
    ) -> Result<(Vec<u8>, Vec<u8>), MixProcessingError> {
        let ack_len = SurbAck::len(Some(packet_type));

        // in theory it's impossible for this to fail since it managed to go into correct `match`
        // branch at the caller
        if extracted_data.len() < ack_len {
            return Err(MixProcessingError::NoSurbAckInFinalHop);
        }

        let message = extracted_data.split_off(ack_len);
        let ack_data = extracted_data;
        Ok((ack_data, message))
    }

    /// Tries to extract a SURBAck that could be sent back into the mix network and message
    /// that should get delivered to a client from received Sphinx packet.
    fn split_into_ack_and_message(
        &self,
        data: Vec<u8>,
        packet_size: PacketSize,
        packet_type: PacketType,
    ) -> Result<(Option<MixPacket>, Vec<u8>), MixProcessingError> {
        match packet_size {
            PacketSize::AckPacket | PacketSize::OutfoxAckPacket => {
                trace!("received an ack packet!");
                Ok((None, data))
            }
            PacketSize::RegularPacket
            | PacketSize::ExtendedPacket8
            | PacketSize::ExtendedPacket16
            | PacketSize::ExtendedPacket32
            | PacketSize::OutfoxRegularPacket => {
                trace!("received a normal packet!");
                let (ack_data, message) =
                    self.split_hop_data_into_ack_and_message(data, packet_type)?;
                let (ack_first_hop, ack_packet) =
                    match SurbAck::try_recover_first_hop_packet(&ack_data, packet_type) {
                        Ok((first_hop, packet)) => (first_hop, packet),
                        Err(err) => {
                            info!("Failed to recover first hop from ack data: {err}");
                            return Err(err.into());
                        }
                    };
                let forward_ack = MixPacket::new(ack_first_hop, ack_packet, packet_type);
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
        payload: Vec<u8>,
        packet_size: PacketSize,
        packet_type: PacketType,
    ) -> Result<MixProcessingResult, MixProcessingError> {
        let (forward_ack, message) =
            self.split_into_ack_and_message(payload, packet_size, packet_type)?;

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
        packet: NymProcessedPacket,
        packet_size: PacketSize,
        packet_type: PacketType,
    ) -> Result<MixProcessingResult, MixProcessingError> {
        match packet {
            NymProcessedPacket::Sphinx(packet) => {
                match packet {
                    ProcessedPacket::ForwardHop(packet, address, delay) => self
                        .process_forward_hop(
                            NymPacket::Sphinx(*packet),
                            address,
                            delay,
                            packet_type,
                        ),
                    // right now there's no use for the surb_id included in the header - probably it should get removed from the
                    // sphinx all together?
                    ProcessedPacket::FinalHop(destination, _, payload) => self.process_final_hop(
                        destination,
                        payload.recover_plaintext()?,
                        packet_size,
                        packet_type,
                    ),
                }
            }
            NymProcessedPacket::Outfox(packet) => {
                let next_address = *packet.next_address();
                let packet = packet.into_packet();
                if packet.is_final_hop() {
                    self.process_final_hop(
                        DestinationAddressBytes::from_bytes(next_address),
                        packet.recover_plaintext()?.to_vec(),
                        packet_size,
                        packet_type,
                    )
                } else {
                    let mix_packet = MixPacket::new(
                        NymNodeRoutingAddress::try_from_bytes(&next_address)?,
                        NymPacket::Outfox(packet),
                        PacketType::Outfox,
                    );
                    Ok(MixProcessingResult::ForwardHop(mix_packet, None))
                }
            }
        }
    }

    pub fn process_received(
        &self,
        received: FramedNymPacket,
    ) -> Result<MixProcessingResult, MixProcessingError> {
        // explicit packet size will help to correctly parse final hop
        nanos!("process_received", {
            let packet_size = received.packet_size();
            let packet_type = received.packet_type();

            // unwrap the sphinx packet and if possible and appropriate, cache keys
            let processed_packet = self.perform_initial_unwrapping(received)?;

            // for forward packets, extract next hop and set delay (but do NOT delay here)
            // for final packets, extract SURBAck
            let final_processing_result =
                self.perform_final_processing(processed_packet, packet_size, packet_type);

            if final_processing_result.is_err() {
                error!("{:?}", final_processing_result)
            }

            final_processing_result
        })
    }
}

// TODO: what more could we realistically test here?
#[cfg(test)]
mod tests {
    use super::*;
    use nym_sphinx_types::crypto::keygen;

    fn fixture() -> SphinxPacketProcessor {
        let local_keys = keygen();
        SphinxPacketProcessor::new(local_keys.0)
    }

    #[tokio::test]
    async fn splitting_hop_data_works_for_sufficiently_long_payload() {
        let processor = fixture();

        let short_data = vec![42u8];
        assert!(processor
            .split_hop_data_into_ack_and_message(short_data, PacketType::Mix)
            .is_err());

        let sufficient_data = vec![42u8; SurbAck::len(Some(PacketType::Mix))];
        let (ack, data) = processor
            .split_hop_data_into_ack_and_message(sufficient_data.clone(), PacketType::Mix)
            .unwrap();
        assert_eq!(sufficient_data, ack);
        assert!(data.is_empty());

        let long_data = vec![42u8; SurbAck::len(Some(PacketType::Mix)) * 5];
        let (ack, data) = processor
            .split_hop_data_into_ack_and_message(long_data, PacketType::Mix)
            .unwrap();
        assert_eq!(ack.len(), SurbAck::len(Some(PacketType::Mix)));
        assert_eq!(data.len(), SurbAck::len(Some(PacketType::Mix)) * 4)
    }

    #[tokio::test]
    async fn splitting_hop_data_works_for_sufficiently_long_payload_outfox() {
        let processor = fixture();

        let short_data = vec![42u8];
        assert!(processor
            .split_hop_data_into_ack_and_message(short_data, PacketType::Outfox)
            .is_err());

        let sufficient_data = vec![42u8; SurbAck::len(Some(PacketType::Outfox))];
        let (ack, data) = processor
            .split_hop_data_into_ack_and_message(sufficient_data.clone(), PacketType::Outfox)
            .unwrap();
        assert_eq!(sufficient_data, ack);
        assert!(data.is_empty());

        let long_data = vec![42u8; SurbAck::len(Some(PacketType::Outfox)) * 5];
        let (ack, data) = processor
            .split_hop_data_into_ack_and_message(long_data, PacketType::Outfox)
            .unwrap();
        assert_eq!(ack.len(), SurbAck::len(Some(PacketType::Outfox)));
        assert_eq!(data.len(), SurbAck::len(Some(PacketType::Outfox)) * 4)
    }

    #[tokio::test]
    async fn splitting_into_ack_and_message_returns_whole_data_for_ack() {
        let processor = fixture();

        let data = vec![42u8; SurbAck::len(Some(PacketType::Mix)) + 10];
        let (ack, message) = processor
            .split_into_ack_and_message(data.clone(), PacketSize::AckPacket, PacketType::Mix)
            .unwrap();
        assert!(ack.is_none());
        assert_eq!(data, message)
    }

    #[tokio::test]
    async fn splitting_into_ack_and_message_returns_whole_data_for_ack_outfox() {
        let processor = fixture();

        let data = vec![42u8; SurbAck::len(Some(PacketType::Outfox)) + 10];
        let (ack, message) = processor
            .split_into_ack_and_message(
                data.clone(),
                PacketSize::OutfoxAckPacket,
                PacketType::Outfox,
            )
            .unwrap();
        assert!(ack.is_none());
        assert_eq!(data, message)
    }
}

// Copyright 2021-2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::packet::FramedNymPacket;
use nym_sphinx_acknowledgements::surb_ack::{SurbAck, SurbAckRecoveryError};
use nym_sphinx_addressing::nodes::{NymNodeRoutingAddress, NymNodeRoutingAddressError};
use nym_sphinx_forwarding::packet::MixPacket;
use nym_sphinx_params::{PacketSize, PacketType, SphinxKeyRotation};
use nym_sphinx_types::header::shared_secret::ExpandedSharedSecret;
use nym_sphinx_types::{
    Delay as SphinxDelay, DestinationAddressBytes, NodeAddressBytes, NymPacket, NymPacketError,
    NymProcessedPacket, PrivateKey, ProcessedPacketData, REPLAY_TAG_SIZE, SphinxError,
    Version as SphinxPacketVersion,
};
use std::fmt::Display;
use thiserror::Error;
use tracing::{debug, info, trace};

#[derive(Debug)]
pub enum MixProcessingResultData {
    /// Contains unwrapped data that should first get delayed before being sent to next hop.
    ForwardHop {
        packet: MixPacket,
        delay: Option<SphinxDelay>,
    },

    /// Contains all data extracted out of the final hop packet that could be forwarded to the destination.
    FinalHop { final_hop_data: ProcessedFinalHop },
}

#[derive(Debug, Copy, Clone)]
pub enum MixPacketVersion {
    Outfox,
    Sphinx(SphinxPacketVersion),
}

impl Display for MixPacketVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            MixPacketVersion::Outfox => "outfox".fmt(f),
            MixPacketVersion::Sphinx(sphinx_version) => {
                write!(f, "sphinx-{}", sphinx_version.value())
            }
        }
    }
}

#[derive(Debug)]
pub struct MixProcessingResult {
    pub packet_version: MixPacketVersion,
    pub processing_data: MixProcessingResultData,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum PartialMixProcessingResult {
    Sphinx {
        expanded_shared_secret: ExpandedSharedSecret,
    },
    Outfox,
}

impl PartialMixProcessingResult {
    pub fn replay_tag(&self) -> Option<&[u8; REPLAY_TAG_SIZE]> {
        match self {
            PartialMixProcessingResult::Sphinx {
                expanded_shared_secret,
            } => Some(expanded_shared_secret.replay_tag()),
            PartialMixProcessingResult::Outfox => None,
        }
    }
}

type ForwardAck = MixPacket;

#[derive(Debug)]
pub struct ProcessedFinalHop {
    pub destination: DestinationAddressBytes,
    pub forward_ack: Option<ForwardAck>,
    pub message: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum PacketProcessingError {
    #[error("failed to process received packet: {0}")]
    NymPacketProcessingError(#[from] NymPacketError),

    #[error("failed to process received sphinx packet: {0}")]
    SphinxProcessingError(#[from] SphinxError),

    #[error("the forward hop address was malformed: {0}")]
    InvalidForwardHopAddress(#[from] NymNodeRoutingAddressError),

    #[error("the final hop did not contain a SURB-Ack")]
    NoSurbAckInFinalHop,

    #[error("failed to recover the expected SURB-Ack packet: {0}")]
    MalformedSurbAck(#[from] SurbAckRecoveryError),

    #[error("attempted to partially process an outfox packet")]
    PartialOutfoxProcessing,

    #[error("the key needed for unwrapping this packet has already expired")]
    ExpiredKey,

    #[error("this packet has already been processed before")]
    PacketReplay,

    #[error("encountered a packet type that is not currently supported by the network")]
    UnsupportedPacketType,
}

impl PacketProcessingError {
    pub fn is_replay(&self) -> bool {
        matches!(self, PacketProcessingError::PacketReplay)
    }
}

pub struct PartialyUnwrappedPacketWithKeyRotation {
    pub packet: PartiallyUnwrappedPacket,
    pub used_key_rotation: u32,
}

pub struct PartiallyUnwrappedPacket {
    received_data: FramedNymPacket,
    partial_result: PartialMixProcessingResult,
}

impl PartiallyUnwrappedPacket {
    /// Attempt to partially unwrap received packet to derive relevant keys
    /// to allow us to reject it for obvious bad behaviour (like replay or invalid mac)
    /// without performing full processing
    pub fn new(
        received_data: FramedNymPacket,
        sphinx_key: &PrivateKey,
    ) -> Result<Self, (FramedNymPacket, PacketProcessingError)> {
        let partial_result = match received_data.packet() {
            NymPacket::Sphinx(packet) => {
                let expanded_shared_secret =
                    packet.header.compute_expanded_shared_secret(sphinx_key);

                // don't continue if the header is malformed
                if let Err(err) = packet
                    .header
                    .ensure_header_integrity(&expanded_shared_secret)
                {
                    return Err((received_data, err.into()));
                }

                PartialMixProcessingResult::Sphinx {
                    expanded_shared_secret,
                }
            }
            _ => return Err((received_data, PacketProcessingError::UnsupportedPacketType)),
        };
        Ok(PartiallyUnwrappedPacket {
            received_data,
            partial_result,
        })
    }

    pub fn finalise_unwrapping(self) -> Result<MixProcessingResult, PacketProcessingError> {
        let packet_size = self.received_data.packet_size();
        let packet_type = self.received_data.packet_type();

        let key_rotation = self.received_data.header.key_rotation;
        let packet = self.received_data.into_inner();

        // currently partial unwrapping is only implemented for sphinx packets.
        // attempting to call it for anything else should result in a failure
        let (
            NymPacket::Sphinx(packet),
            PartialMixProcessingResult::Sphinx {
                expanded_shared_secret,
            },
        ) = (packet, self.partial_result)
        else {
            return Err(PacketProcessingError::PartialOutfoxProcessing);
        };
        let processed_packet = packet.process_with_expanded_secret(&expanded_shared_secret)?;
        wrap_processed_sphinx_packet(processed_packet, packet_size, packet_type, key_rotation)
    }

    pub fn replay_tag(&self) -> Option<&[u8; REPLAY_TAG_SIZE]> {
        self.partial_result.replay_tag()
    }

    pub fn with_key_rotation(
        self,
        used_key_rotation: u32,
    ) -> PartialyUnwrappedPacketWithKeyRotation {
        PartialyUnwrappedPacketWithKeyRotation {
            packet: self,
            used_key_rotation,
        }
    }
}

impl From<(FramedNymPacket, PartialMixProcessingResult)> for PartiallyUnwrappedPacket {
    fn from(
        (received_data, partial_result): (FramedNymPacket, PartialMixProcessingResult),
    ) -> Self {
        PartiallyUnwrappedPacket {
            received_data,
            partial_result,
        }
    }
}

pub fn process_framed_packet(
    received: FramedNymPacket,
    sphinx_key: &PrivateKey,
) -> Result<MixProcessingResult, PacketProcessingError> {
    let packet_size = received.packet_size();
    let packet_type = received.packet_type();
    let key_rotation = received.key_rotation();

    // unwrap the sphinx packet
    let processed_packet = perform_framed_unwrapping(received, sphinx_key)?;

    // for forward packets, extract next hop and set delay (but do NOT delay here)
    // for final packets, extract SURBAck
    perform_final_processing(processed_packet, packet_size, packet_type, key_rotation)
}

fn perform_framed_unwrapping(
    received: FramedNymPacket,
    sphinx_key: &PrivateKey,
) -> Result<NymProcessedPacket, PacketProcessingError> {
    let packet = received.into_inner();
    perform_framed_packet_processing(packet, sphinx_key)
}

fn perform_framed_packet_processing(
    packet: NymPacket,
    sphinx_key: &PrivateKey,
) -> Result<NymProcessedPacket, PacketProcessingError> {
    packet.process(sphinx_key).map_err(|err| {
        debug!("Failed to unwrap NymPacket packet: {err}");
        PacketProcessingError::NymPacketProcessingError(err)
    })
}

// r[impl delay.only-hop-reads]
fn wrap_processed_sphinx_packet(
    packet: nym_sphinx_types::ProcessedPacket,
    packet_size: PacketSize,
    packet_type: PacketType,
    key_rotation: SphinxKeyRotation,
) -> Result<MixProcessingResult, PacketProcessingError> {
    let processing_data = match packet.data {
        ProcessedPacketData::ForwardHop {
            next_hop_packet,
            next_hop_address,
            delay,
        } => process_forward_hop(
            NymPacket::Sphinx(next_hop_packet),
            next_hop_address,
            delay,
            packet_type,
            key_rotation,
        ),
        // right now there's no use for the surb_id included in the header - probably it should get removed from the
        // sphinx all together?
        ProcessedPacketData::FinalHop {
            destination,
            identifier: _,
            payload,
        } => process_final_hop(
            destination,
            payload.recover_plaintext()?,
            packet_size,
            packet_type,
            key_rotation,
        ),
    }?;

    Ok(MixProcessingResult {
        packet_version: MixPacketVersion::Sphinx(packet.version),
        processing_data,
    })
}

fn perform_final_processing(
    packet: NymProcessedPacket,
    packet_size: PacketSize,
    packet_type: PacketType,
    key_rotation: SphinxKeyRotation,
) -> Result<MixProcessingResult, PacketProcessingError> {
    match packet {
        NymProcessedPacket::Sphinx(packet) => {
            wrap_processed_sphinx_packet(packet, packet_size, packet_type, key_rotation)
        }
        _ => Err(PacketProcessingError::UnsupportedPacketType),
    }
}

fn process_final_hop(
    destination: DestinationAddressBytes,
    payload: Vec<u8>,
    packet_size: PacketSize,
    packet_type: PacketType,
    key_rotation: SphinxKeyRotation,
) -> Result<MixProcessingResultData, PacketProcessingError> {
    let (forward_ack, message) =
        split_into_ack_and_message(payload, packet_size, packet_type, key_rotation)?;

    Ok(MixProcessingResultData::FinalHop {
        final_hop_data: ProcessedFinalHop {
            destination,
            forward_ack,
            message,
        },
    })
}

// r[impl sphinx.ack-packet]
fn split_into_ack_and_message(
    data: Vec<u8>,
    packet_size: PacketSize,
    packet_type: PacketType,
    key_rotation: SphinxKeyRotation,
) -> Result<(Option<MixPacket>, Vec<u8>), PacketProcessingError> {
    #[allow(deprecated)]
    match packet_size {
        PacketSize::OutfoxAckPacket | PacketSize::OutfoxRegularPacket => {
            Err(PacketProcessingError::UnsupportedPacketType)
        }
        PacketSize::AckPacket => {
            trace!("received an ack packet!");
            Ok((None, data))
        }
        PacketSize::RegularPacket
        | PacketSize::ExtendedPacket8
        | PacketSize::ExtendedPacket16
        | PacketSize::ExtendedPacket32 => {
            trace!("received a normal packet!");
            let (ack_data, message) = split_hop_data_into_ack_and_message(data, packet_type)?;
            let (ack_first_hop, ack_packet) =
                match SurbAck::try_recover_first_hop_packet(&ack_data, packet_type) {
                    Ok((first_hop, packet)) => (first_hop, packet),
                    Err(err) => {
                        info!("Failed to recover first hop from ack data: {err}");
                        return Err(err.into());
                    }
                };
            let forward_ack = MixPacket::new(ack_first_hop, ack_packet, packet_type, key_rotation);
            Ok((Some(forward_ack), message))
        }
        _ => Err(PacketProcessingError::UnsupportedPacketType),
    }
}

#[allow(dead_code)]
fn split_hop_data_into_ack_and_message(
    mut extracted_data: Vec<u8>,
    packet_type: PacketType,
) -> Result<(Vec<u8>, Vec<u8>), PacketProcessingError> {
    let ack_len = SurbAck::len(Some(packet_type));

    // in theory it's impossible for this to fail since it managed to go into correct `match`
    // branch at the caller
    if extracted_data.len() < ack_len {
        return Err(PacketProcessingError::NoSurbAckInFinalHop);
    }

    let message = extracted_data.split_off(ack_len);
    let ack_data = extracted_data;
    Ok((ack_data, message))
}

// r[impl sphinx.forward-hop]
fn process_forward_hop(
    packet: NymPacket,
    forward_address: NodeAddressBytes,
    delay: SphinxDelay,
    packet_type: PacketType,
    key_rotation: SphinxKeyRotation,
) -> Result<MixProcessingResultData, PacketProcessingError> {
    let next_hop_address = NymNodeRoutingAddress::try_from(forward_address)?;

    let packet = MixPacket::new(next_hop_address, packet, packet_type, key_rotation);
    Ok(MixProcessingResultData::ForwardHop {
        packet,
        delay: Some(delay),
    })
}

// TODO: what more could we realistically test here?
#[cfg(test)]
mod tests {
    use super::*;
    use nym_sphinx_addressing::nodes::MAX_NODE_ADDRESS_UNPADDED_LEN;
    use nym_sphinx_types::{
        DESTINATION_ADDRESS_LENGTH, Destination, HEADER_SIZE, IDENTIFIER_LENGTH, Node,
        PAYLOAD_KEYS_SEEDS_VERSION, PublicKey,
    };
    use std::net::SocketAddr;

    fn hop_address(hop: u8) -> NymNodeRoutingAddress {
        SocketAddr::from(([10, 0, 0, hop], 1789)).into()
    }

    fn hop_key(hop: u8) -> PrivateKey {
        PrivateKey::from([hop; 32])
    }

    /// An ack-sized packet through hops 1 to 4, so that hop 4 is the final hop and returns the
    /// payload without looking for a SURB-ACK in it.
    fn framed_packet_through_four_hops(delays: &[SphinxDelay; 4]) -> FramedNymPacket {
        let route: Vec<_> = (1..=4)
            .map(|hop| {
                Node::new(
                    hop_address(hop).try_into().unwrap(),
                    PublicKey::from(&hop_key(hop)),
                )
            })
            .collect();
        let destination = Destination::new(
            DestinationAddressBytes::from_bytes([3u8; DESTINATION_ADDRESS_LENGTH]),
            [4u8; IDENTIFIER_LENGTH],
        );
        let packet = NymPacket::sphinx_build_legacy(
            PacketSize::AckPacket.payload_size(),
            b"foomp",
            &route,
            &destination,
            delays,
        )
        .unwrap();
        FramedNymPacket::new(
            packet,
            PacketType::Mix,
            SphinxKeyRotation::EvenRotation,
            false,
        )
    }

    fn reframe(packet: &MixPacket) -> FramedNymPacket {
        let bytes = packet.packet().to_bytes().unwrap();
        FramedNymPacket::new(
            NymPacket::sphinx_from_bytes(&bytes).unwrap(),
            PacketType::Mix,
            SphinxKeyRotation::EvenRotation,
            false,
        )
    }

    // r[verify sphinx.forward-hop]
    #[test]
    fn a_forward_hop_learns_its_next_hop_and_delay_and_drops_a_tampered_header() {
        let delays = [11, 22, 33, 44].map(SphinxDelay::new_from_millis);
        let received = framed_packet_through_four_hops(&delays);
        let received_bytes = received.packet().to_bytes().unwrap();

        let mut tampered_bytes = received_bytes.clone();
        // the first byte of the encrypted routing information, after the group element and the MAC
        tampered_bytes[32 + 16] ^= 1;
        let tampered = FramedNymPacket::new(
            NymPacket::sphinx_from_bytes(&tampered_bytes).unwrap(),
            PacketType::Mix,
            SphinxKeyRotation::EvenRotation,
            false,
        );
        let Err((_, err)) = PartiallyUnwrappedPacket::new(tampered, &hop_key(1)) else {
            panic!("a tampered header passed the MAC check");
        };
        assert!(matches!(
            err,
            PacketProcessingError::SphinxProcessingError(_)
        ));

        let processed = PartiallyUnwrappedPacket::new(received, &hop_key(1))
            .unwrap()
            .finalise_unwrapping()
            .unwrap();
        let MixPacketVersion::Sphinx(version) = processed.packet_version else {
            panic!("expected a sphinx packet");
        };
        assert_eq!(version, PAYLOAD_KEYS_SEEDS_VERSION);
        let MixProcessingResultData::ForwardHop { packet, delay } = processed.processing_data
        else {
            panic!("expected a forward hop");
        };
        assert_eq!(packet.next_hop(), hop_address(2));
        assert_eq!(delay, Some(delays[0]));

        let forwarded_bytes = packet.packet().to_bytes().unwrap();
        let forwarded_header = &packet.packet().sphinx_packet_ref().unwrap().header;
        assert_eq!(HEADER_SIZE, 348);
        assert_eq!(forwarded_header.to_bytes().len(), HEADER_SIZE);
        assert_eq!(forwarded_bytes.len(), received_bytes.len());
        // the group element is blinded before the packet is forwarded
        assert_ne!(forwarded_bytes[..32], received_bytes[..32]);
    }

    // r[verify delay.only-hop-reads]
    #[test]
    fn each_mix_reads_only_its_own_delay_and_no_link_carries_a_delay_in_clear() {
        let delays = [
            12_345_678_901,
            23_456_789_012,
            34_567_890_123,
            45_678_901_234,
        ]
        .map(SphinxDelay::new_from_nanos);
        let assert_no_delay_in_clear = |framed: &FramedNymPacket| {
            let wire_bytes = framed.packet().to_bytes().unwrap();
            for delay in &delays {
                assert!(
                    !wire_bytes.windows(8).any(|w| w == delay.to_bytes()),
                    "{delay:?} is in clear on a link"
                );
            }
        };

        let mut framed = framed_packet_through_four_hops(&delays);
        for hop in 1..=3u8 {
            assert_no_delay_in_clear(&framed);
            let processed = process_framed_packet(framed, &hop_key(hop)).unwrap();
            let MixProcessingResultData::ForwardHop { packet, delay } = processed.processing_data
            else {
                panic!("expected a forward hop at hop {hop}");
            };
            assert_eq!(delay, Some(delays[hop as usize - 1]));
            // a hop cannot open the layer of the next hop, so it cannot read the next delay
            assert!(process_framed_packet(reframe(&packet), &hop_key(hop)).is_err());
            framed = reframe(&packet);
        }

        assert_no_delay_in_clear(&framed);
        let processed = process_framed_packet(framed, &hop_key(4)).unwrap();
        assert!(matches!(
            processed.processing_data,
            MixProcessingResultData::FinalHop { .. }
        ));
    }

    // r[verify sphinx.ack-packet]
    #[test]
    fn the_egress_forwards_the_surb_ack_to_its_first_hop_and_keeps_the_message() {
        let delays = [11, 22, 33, 44].map(SphinxDelay::new_from_millis);
        let ack_bytes = framed_packet_through_four_hops(&delays)
            .into_inner()
            .to_bytes()
            .unwrap();
        assert_eq!(ack_bytes.len(), PacketSize::AckPacket.size());
        let message = b"the message".to_vec();
        let data: Vec<u8> = hop_address(1)
            .as_zero_padded_bytes(MAX_NODE_ADDRESS_UNPADDED_LEN)
            .into_iter()
            .chain(ack_bytes.clone())
            .chain(message.clone())
            .collect();

        let (forward_ack, recovered_message) = split_into_ack_and_message(
            data,
            PacketSize::RegularPacket,
            PacketType::Mix,
            SphinxKeyRotation::EvenRotation,
        )
        .unwrap();

        let forward_ack = forward_ack.unwrap();
        assert_eq!(forward_ack.next_hop(), hop_address(1));
        assert_eq!(forward_ack.packet().to_bytes().unwrap(), ack_bytes);
        assert_eq!(recovered_message, message);
    }

    #[test]
    fn splitting_hop_data_works_for_sufficiently_long_payload() {
        let short_data = vec![42u8];
        assert!(split_hop_data_into_ack_and_message(short_data, PacketType::Mix).is_err());

        let sufficient_data = vec![42u8; SurbAck::len(Some(PacketType::Mix))];
        let (ack, data) =
            split_hop_data_into_ack_and_message(sufficient_data.clone(), PacketType::Mix).unwrap();
        assert_eq!(sufficient_data, ack);
        assert!(data.is_empty());

        let long_data: Vec<u8> = vec![42u8; SurbAck::len(Some(PacketType::Mix)) * 5];
        let (ack, data) = split_hop_data_into_ack_and_message(long_data, PacketType::Mix).unwrap();
        assert_eq!(ack.len(), SurbAck::len(Some(PacketType::Mix)));
        assert_eq!(data.len(), SurbAck::len(Some(PacketType::Mix)) * 4)
    }

    #[test]
    fn splitting_into_ack_and_message_returns_whole_data_for_ack() {
        let data = vec![42u8; SurbAck::len(Some(PacketType::Mix)) + 10];
        let (ack, message) = split_into_ack_and_message(
            data.clone(),
            PacketSize::AckPacket,
            PacketType::Mix,
            SphinxKeyRotation::EvenRotation,
        )
        .unwrap();
        assert!(ack.is_none());
        assert_eq!(data, message)
    }
}

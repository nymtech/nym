// Copyright 2021-2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::packet::FramedNymPacket;
use log::{debug, error, info, trace};
use nym_sphinx_acknowledgements::surb_ack::{SurbAck, SurbAckRecoveryError};
use nym_sphinx_addressing::nodes::{NymNodeRoutingAddress, NymNodeRoutingAddressError};
use nym_sphinx_forwarding::packet::MixPacket;
use nym_sphinx_params::{PacketSize, PacketType};
use nym_sphinx_types::header::shared_secret::ExpandedSharedSecret;
use nym_sphinx_types::{
    Delay as SphinxDelay, DestinationAddressBytes, NodeAddressBytes, NymPacket, NymPacketError,
    NymProcessedPacket, OutfoxError, OutfoxProcessedPacket, PrivateKey, ProcessedPacketData,
    SphinxError, Version as SphinxPacketVersion, REPLAY_TAG_SIZE,
};
use std::fmt::Display;
use thiserror::Error;

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

    #[error("failed to process received outfox packet: {0}")]
    OutfoxProcessingError(#[from] OutfoxError),

    #[error("attempted to partially process an outfox packet")]
    PartialOutfoxProcessing,

    #[error("this packet has already been processed before")]
    PacketReplay,
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
    ) -> Result<Self, PacketProcessingError> {
        let partial_result = match received_data.packet() {
            NymPacket::Sphinx(packet) => {
                let expanded_shared_secret =
                    packet.header.compute_expanded_shared_secret(sphinx_key);

                // don't continue if the header is malformed
                packet
                    .header
                    .ensure_header_integrity(&expanded_shared_secret)?;

                PartialMixProcessingResult::Sphinx {
                    expanded_shared_secret,
                }
            }

            NymPacket::Outfox(_) => PartialMixProcessingResult::Outfox,
        };
        Ok(PartiallyUnwrappedPacket {
            received_data,
            partial_result,
        })
    }

    pub fn finalise_unwrapping(self) -> Result<MixProcessingResult, PacketProcessingError> {
        let packet_size = self.received_data.packet_size();
        let packet_type = self.received_data.packet_type();

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
        wrap_processed_sphinx_packet(processed_packet, packet_size, packet_type)
    }

    pub fn replay_tag(&self) -> Option<&[u8; REPLAY_TAG_SIZE]> {
        self.partial_result.replay_tag()
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

    // unwrap the sphinx packet
    let processed_packet = perform_framed_unwrapping(received, sphinx_key)?;

    // for forward packets, extract next hop and set delay (but do NOT delay here)
    // for final packets, extract SURBAck
    perform_final_processing(processed_packet, packet_size, packet_type)
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

fn wrap_processed_sphinx_packet(
    packet: nym_sphinx_types::ProcessedPacket,
    packet_size: PacketSize,
    packet_type: PacketType,
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
        ),
    }?;

    Ok(MixProcessingResult {
        packet_version: MixPacketVersion::Sphinx(packet.version),
        processing_data,
    })
}

fn wrap_processed_outfox_packet(
    packet: OutfoxProcessedPacket,
    packet_size: PacketSize,
    packet_type: PacketType,
) -> Result<MixProcessingResult, PacketProcessingError> {
    let next_address = *packet.next_address();
    let packet = packet.into_packet();
    if packet.is_final_hop() {
        let processing_data = process_final_hop(
            DestinationAddressBytes::from_bytes(next_address),
            packet.recover_plaintext()?.to_vec(),
            packet_size,
            packet_type,
        )?;
        Ok(MixProcessingResult {
            packet_version: MixPacketVersion::Outfox,
            processing_data,
        })
    } else {
        let packet = MixPacket::new(
            NymNodeRoutingAddress::try_from_bytes(&next_address)?,
            NymPacket::Outfox(packet),
            PacketType::Outfox,
        );
        Ok(MixProcessingResult {
            packet_version: MixPacketVersion::Outfox,
            processing_data: MixProcessingResultData::ForwardHop {
                packet,
                delay: None,
            },
        })
    }
}

fn perform_final_processing(
    packet: NymProcessedPacket,
    packet_size: PacketSize,
    packet_type: PacketType,
) -> Result<MixProcessingResult, PacketProcessingError> {
    match packet {
        NymProcessedPacket::Sphinx(packet) => {
            wrap_processed_sphinx_packet(packet, packet_size, packet_type)
        }
        NymProcessedPacket::Outfox(packet) => {
            wrap_processed_outfox_packet(packet, packet_size, packet_type)
        }
    }
}

fn process_final_hop(
    destination: DestinationAddressBytes,
    payload: Vec<u8>,
    packet_size: PacketSize,
    packet_type: PacketType,
) -> Result<MixProcessingResultData, PacketProcessingError> {
    let (forward_ack, message) = split_into_ack_and_message(payload, packet_size, packet_type)?;

    Ok(MixProcessingResultData::FinalHop {
        final_hop_data: ProcessedFinalHop {
            destination,
            forward_ack,
            message,
        },
    })
}

fn split_into_ack_and_message(
    data: Vec<u8>,
    packet_size: PacketSize,
    packet_type: PacketType,
) -> Result<(Option<MixPacket>, Vec<u8>), PacketProcessingError> {
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
            let (ack_data, message) = split_hop_data_into_ack_and_message(data, packet_type)?;
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

fn process_forward_hop(
    packet: NymPacket,
    forward_address: NodeAddressBytes,
    delay: SphinxDelay,
    packet_type: PacketType,
) -> Result<MixProcessingResultData, PacketProcessingError> {
    let next_hop_address = NymNodeRoutingAddress::try_from(forward_address)?;

    let packet = MixPacket::new(next_hop_address, packet, packet_type);
    Ok(MixProcessingResultData::ForwardHop {
        packet,
        delay: Some(delay),
    })
}

// TODO: what more could we realistically test here?
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn splitting_hop_data_works_for_sufficiently_long_payload() {
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

    #[tokio::test]
    async fn splitting_hop_data_works_for_sufficiently_long_payload_outfox() {
        let short_data = vec![42u8];
        assert!(split_hop_data_into_ack_and_message(short_data, PacketType::Outfox).is_err());

        let sufficient_data = vec![42u8; SurbAck::len(Some(PacketType::Outfox))];
        let (ack, data) =
            split_hop_data_into_ack_and_message(sufficient_data.clone(), PacketType::Outfox)
                .unwrap();
        assert_eq!(sufficient_data, ack);
        assert!(data.is_empty());

        let long_data = vec![42u8; SurbAck::len(Some(PacketType::Outfox)) * 5];
        let (ack, data) =
            split_hop_data_into_ack_and_message(long_data, PacketType::Outfox).unwrap();
        assert_eq!(ack.len(), SurbAck::len(Some(PacketType::Outfox)));
        assert_eq!(data.len(), SurbAck::len(Some(PacketType::Outfox)) * 4)
    }

    #[tokio::test]
    async fn splitting_into_ack_and_message_returns_whole_data_for_ack() {
        let data = vec![42u8; SurbAck::len(Some(PacketType::Mix)) + 10];
        let (ack, message) =
            split_into_ack_and_message(data.clone(), PacketSize::AckPacket, PacketType::Mix)
                .unwrap();
        assert!(ack.is_none());
        assert_eq!(data, message)
    }

    #[tokio::test]
    async fn splitting_into_ack_and_message_returns_whole_data_for_ack_outfox() {
        let data = vec![42u8; SurbAck::len(Some(PacketType::Outfox)) + 10];
        let (ack, message) = split_into_ack_and_message(
            data.clone(),
            PacketSize::OutfoxAckPacket,
            PacketType::Outfox,
        )
        .unwrap();
        assert!(ack.is_none());
        assert_eq!(data, message)
    }
}

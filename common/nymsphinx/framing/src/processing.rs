use log::{debug, error, info, trace};
use nym_sphinx_acknowledgements::surb_ack::{SurbAck, SurbAckRecoveryError};
use nym_sphinx_addressing::nodes::{NymNodeRoutingAddress, NymNodeRoutingAddressError};
use nym_sphinx_params::{PacketSize, PacketType};
use nym_sphinx_types::{
    Delay as SphinxDelay, DestinationAddressBytes, NodeAddressBytes, NymPacket, NymPacketError,
    NymProcessedPacket, OutfoxError, PrivateKey, ProcessedPacket, SphinxError,
};
use thiserror::Error;

use crate::packet::FramedNymPacket;
use nym_metrics::nanos;
use nym_sphinx_forwarding::packet::MixPacket;

#[derive(Debug)]
pub enum MixProcessingResult {
    /// Contains unwrapped data that should first get delayed before being sent to next hop.
    ForwardHop(MixPacket, Option<SphinxDelay>),

    /// Contains all data extracted out of the final hop packet that could be forwarded to the destination.
    FinalHop(ProcessedFinalHop),
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

    #[error("the received packet was set to use the very old and very much deprecated 'VPN' mode")]
    ReceivedOldTypeVpnPacket,

    #[error("failed to process received outfox packet: {0}")]
    OutfoxProcessingError(#[from] OutfoxError),
}

pub fn process_framed_packet(
    received: FramedNymPacket,
    sphinx_key: &PrivateKey,
) -> Result<MixProcessingResult, PacketProcessingError> {
    nanos!("process_received", {
        let packet_size = received.packet_size();
        let packet_type = received.packet_type();

        // unwrap the sphinx packet and if possible and appropriate, cache keys
        let processed_packet = perform_framed_unwrapping(received, sphinx_key)?;

        // for forward packets, extract next hop and set delay (but do NOT delay here)
        // for final packets, extract SURBAck
        let final_processing_result =
            perform_final_processing(processed_packet, packet_size, packet_type);

        if final_processing_result.is_err() {
            error!("{:?}", final_processing_result)
        }

        final_processing_result
    })
}

fn perform_framed_unwrapping(
    received: FramedNymPacket,
    sphinx_key: &PrivateKey,
) -> Result<NymProcessedPacket, PacketProcessingError> {
    nanos!("perform_initial_unwrapping", {
        let packet = received.into_inner();
        perform_framed_packet_processing(packet, sphinx_key)
    })
}

fn perform_framed_packet_processing(
    packet: NymPacket,
    sphinx_key: &PrivateKey,
) -> Result<NymProcessedPacket, PacketProcessingError> {
    nanos!("perform_initial_packet_processing", {
        packet.process(sphinx_key).map_err(|err| {
            debug!("Failed to unwrap NymPacket packet: {err}");
            PacketProcessingError::NymPacketProcessingError(err)
        })
    })
}

fn perform_final_processing(
    packet: NymProcessedPacket,
    packet_size: PacketSize,
    packet_type: PacketType,
) -> Result<MixProcessingResult, PacketProcessingError> {
    match packet {
        NymProcessedPacket::Sphinx(packet) => {
            match packet {
                ProcessedPacket::ForwardHop(packet, address, delay) => {
                    process_forward_hop(NymPacket::Sphinx(*packet), address, delay, packet_type)
                }
                // right now there's no use for the surb_id included in the header - probably it should get removed from the
                // sphinx all together?
                ProcessedPacket::FinalHop(destination, _, payload) => process_final_hop(
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
                process_final_hop(
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

fn process_final_hop(
    destination: DestinationAddressBytes,
    payload: Vec<u8>,
    packet_size: PacketSize,
    packet_type: PacketType,
) -> Result<MixProcessingResult, PacketProcessingError> {
    let (forward_ack, message) = split_into_ack_and_message(payload, packet_size, packet_type)?;

    Ok(MixProcessingResult::FinalHop(ProcessedFinalHop {
        destination,
        forward_ack,
        message,
    }))
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
) -> Result<MixProcessingResult, PacketProcessingError> {
    let next_hop_address = NymNodeRoutingAddress::try_from(forward_address)?;

    let mix_packet = MixPacket::new(next_hop_address, packet, packet_type);
    Ok(MixProcessingResult::ForwardHop(mix_packet, Some(delay)))
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

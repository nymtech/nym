// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::shared_key::new_ephemeral_shared_key;
use nym_crypto::symmetric::stream_cipher;
use nym_sphinx_acknowledgements::surb_ack::{SurbAck, SurbAckRecoveryError};
use nym_sphinx_acknowledgements::AckKey;
use nym_sphinx_addressing::clients::Recipient;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_chunking::fragment::COVER_FRAG_ID;
use nym_sphinx_forwarding::packet::MixPacket;
use nym_sphinx_params::packet_sizes::PacketSize;
use nym_sphinx_params::{
    PacketEncryptionAlgorithm, PacketHkdfAlgorithm, PacketType, DEFAULT_NUM_MIX_HOPS,
};
use nym_sphinx_types::NymPacket;
use nym_topology::{NymTopology, NymTopologyError};
use rand::{CryptoRng, RngCore};

use std::time;
use thiserror::Error;

pub const LOOP_COVER_MESSAGE_PAYLOAD: &[u8] = b"The cake is a lie!";

#[derive(Debug, Error)]
pub enum CoverMessageError {
    #[error("Could not construct cover message due to invalid topology - {0}")]
    InvalidTopologyError(#[from] NymTopologyError),

    #[error("SurbAck: {0}")]
    SurbAck(#[from] SurbAckRecoveryError),

    #[error("NymPacket: {0}")]
    NymPacket(#[from] nym_sphinx_types::NymPacketError),
}

pub fn generate_loop_cover_surb_ack<R>(
    rng: &mut R,
    topology: &NymTopology,
    ack_key: &AckKey,
    full_address: &Recipient,
    average_ack_delay: time::Duration,
    packet_type: PacketType,
) -> Result<SurbAck, CoverMessageError>
where
    R: RngCore + CryptoRng,
{
    Ok(SurbAck::construct(
        rng,
        full_address,
        ack_key,
        COVER_FRAG_ID.to_bytes(),
        average_ack_delay,
        topology,
        packet_type,
    )?)
}

#[allow(clippy::too_many_arguments)]
pub fn generate_loop_cover_packet<R>(
    rng: &mut R,
    topology: &NymTopology,
    ack_key: &AckKey,
    full_address: &Recipient,
    average_ack_delay: time::Duration,
    average_packet_delay: time::Duration,
    packet_size: PacketSize,
    packet_type: PacketType,
) -> Result<MixPacket, CoverMessageError>
where
    R: RngCore + CryptoRng,
{
    // we don't care about total ack delay - we will not be retransmitting it anyway
    let (_, ack_bytes) = generate_loop_cover_surb_ack(
        rng,
        topology,
        ack_key,
        full_address,
        average_ack_delay,
        packet_type,
    )?
    .prepare_for_sending()?;

    // cover message can't be distinguishable from a normal traffic so we have to go through
    // all the effort of key generation, encryption, etc. Note here we are generating shared key
    // with ourselves!
    let (ephemeral_keypair, shared_key) = new_ephemeral_shared_key::<
        PacketEncryptionAlgorithm,
        PacketHkdfAlgorithm,
        _,
    >(rng, full_address.encryption_key());

    let public_key_bytes = ephemeral_keypair.public_key().to_bytes();

    let cover_size = packet_size.plaintext_size() - public_key_bytes.len() - ack_bytes.len();

    let mut cover_content: Vec<_> = LOOP_COVER_MESSAGE_PAYLOAD
        .iter()
        .cloned()
        .chain(std::iter::once(1))
        .chain(std::iter::repeat(0))
        .take(cover_size)
        .collect();

    let zero_iv = stream_cipher::zero_iv::<PacketEncryptionAlgorithm>();
    stream_cipher::encrypt_in_place::<PacketEncryptionAlgorithm>(
        &shared_key,
        &zero_iv,
        &mut cover_content,
    );

    // combine it together as follows:
    // SURB_ACK_FIRST_HOP || SURB_ACK_DATA || EPHEMERAL_KEY || COVER_CONTENT
    // (note: surb_ack_bytes contains SURB_ACK_FIRST_HOP || SURB_ACK_DATA )
    let packet_payload: Vec<_> = ack_bytes
        .into_iter()
        .chain(ephemeral_keypair.public_key().to_bytes().iter().cloned())
        .chain(cover_content)
        .collect();

    let route =
        topology.random_route_to_gateway(rng, DEFAULT_NUM_MIX_HOPS, full_address.gateway())?;
    let delays = nym_sphinx_routing::generate_hop_delays(average_packet_delay, route.len());
    let destination = full_address.as_sphinx_destination();

    let first_hop_address =
        NymNodeRoutingAddress::try_from(route.first().unwrap().address).unwrap();

    // once merged, that's an easy rng injection point for sphinx packets : )
    let packet = match packet_type {
        PacketType::Mix => NymPacket::sphinx_build(
            packet_size.payload_size(),
            packet_payload,
            &route,
            &destination,
            &delays,
        )?,
        #[allow(deprecated)]
        PacketType::Vpn => NymPacket::sphinx_build(
            packet_size.payload_size(),
            packet_payload,
            &route,
            &destination,
            &delays,
        )?,
        PacketType::Outfox => NymPacket::outfox_build(
            packet_payload,
            &route,
            &destination,
            Some(packet_size.plaintext_size()),
        )?,
    };

    Ok(MixPacket::new(first_hop_address, packet, packet_type))
}

/// Helper function used to determine if given message represents a loop cover message.
// It kinda seems like there must exist "prefix" or "starts_with" method for bytes
// or something, but I couldn't find anything
pub fn is_cover(data: &[u8]) -> bool {
    if data.len() < LOOP_COVER_MESSAGE_PAYLOAD.len() {
        return false;
    }

    for i in 0..LOOP_COVER_MESSAGE_PAYLOAD.len() {
        if data[i] != LOOP_COVER_MESSAGE_PAYLOAD[i] {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_cover_works_for_identical_input() {
        assert!(is_cover(LOOP_COVER_MESSAGE_PAYLOAD))
    }

    #[test]
    fn is_cover_works_for_longer_input() {
        let input: Vec<_> = LOOP_COVER_MESSAGE_PAYLOAD
            .iter()
            .cloned()
            .chain(std::iter::repeat(42).take(100))
            .collect();
        assert!(is_cover(&input))
    }

    #[test]
    fn is_cover_returns_false_for_unrelated_input() {
        // make sure the length checks out
        let input: Vec<_> = LOOP_COVER_MESSAGE_PAYLOAD.iter().map(|_| 42).collect();
        assert!(!is_cover(&input))
    }

    #[test]
    fn is_cover_returns_false_for_part_of_correct_input() {
        let input: Vec<_> = LOOP_COVER_MESSAGE_PAYLOAD
            .iter()
            .cloned()
            .take(LOOP_COVER_MESSAGE_PAYLOAD.len() - 1)
            .chain(std::iter::once(42))
            .collect();
        assert!(!is_cover(&input))
    }

    #[test]
    fn is_cover_returns_false_for_empty_input() {
        let empty = Vec::new();
        assert!(!is_cover(&empty))
    }
}

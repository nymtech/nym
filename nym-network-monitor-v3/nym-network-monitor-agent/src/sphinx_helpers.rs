// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::test_packet::TestPacketHeader;
use arrayref::array_ref;
use hkdf::Hkdf;
use nym_crypto::aes::cipher::crypto_common::rand_core::OsRng;
use nym_crypto::asymmetric::x25519;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_params::PacketSize;
use nym_sphinx_types::constants::{
    BLINDING_FACTOR_SIZE, EXPANDED_SHARED_SECRET_HKDF_INFO, EXPANDED_SHARED_SECRET_HKDF_SALT,
    EXPANDED_SHARED_SECRET_LENGTH, INTEGRITY_MAC_KEY_SIZE, PAYLOAD_KEY_SEED_SIZE,
};
use nym_sphinx_types::crypto::STREAM_CIPHER_KEY_SIZE;
use nym_sphinx_types::{
    DESTINATION_ADDRESS_LENGTH, Delay, Destination, DestinationAddressBytes, IDENTIFIER_LENGTH,
    Node, PAYLOAD_KEY_SIZE, PayloadKey, SphinxPacket, SphinxPacketBuilder, derive_payload_key,
};
use sha2::Sha256;
use std::net::SocketAddr;
use std::time::Duration;
use x25519_dalek::{PublicKey, StaticSecret};

/// Newtype wrapper around the HKDF-expanded shared secret used in the sphinx protocol
/// since the actual type within the sphinx library does not expose the required methods.
pub(crate) struct ExpandedSharedSecretWrapper(pub(crate) [u8; EXPANDED_SHARED_SECRET_LENGTH]);

impl ExpandedSharedSecretWrapper {
    /// Returns the blinding factor as an x25519 [`StaticSecret`], used to derive the
    /// shared secret for the next hop when manually reconstructing payload keys.
    pub(crate) fn blinding_factor(&self) -> StaticSecret {
        StaticSecret::from(*self.blinding_factor_bytes())
    }

    /// Returns the raw blinding factor bytes.
    pub(crate) fn blinding_factor_bytes(&self) -> &[u8; BLINDING_FACTOR_SIZE] {
        array_ref!(
            &self.0,
            STREAM_CIPHER_KEY_SIZE + INTEGRITY_MAC_KEY_SIZE + PAYLOAD_KEY_SIZE,
            BLINDING_FACTOR_SIZE
        )
    }

    /// Returns the payload key seed, used as input to [`derive_payload_key`].
    pub(crate) fn payload_key_seed(&self) -> &[u8; PAYLOAD_KEY_SEED_SIZE] {
        array_ref!(
            &self.0,
            STREAM_CIPHER_KEY_SIZE + INTEGRITY_MAC_KEY_SIZE,
            PAYLOAD_KEY_SEED_SIZE
        )
    }

    /// Derives the [`PayloadKey`] for this hop from the payload key seed.
    pub(crate) fn derive_payload_key(&self) -> PayloadKey {
        derive_payload_key(self.payload_key_seed())
    }
}

/// Re-derives the expanded shared secret from a raw 32-byte DH shared secret using HKDF-SHA256
/// with the sphinx protocol's standard salt and info strings.
///
/// This mirrors the derivation performed inside the sphinx library, which is not publicly
/// exposed — hence the need to replicate it here when reconstructing payload keys for a
/// reusable header.
pub(crate) fn rederive_expanded_shared_secret(
    shared_secret: &[u8; 32],
) -> ExpandedSharedSecretWrapper {
    let hkdf = Hkdf::<Sha256>::new(Some(EXPANDED_SHARED_SECRET_HKDF_SALT), shared_secret);

    let mut output = [0u8; EXPANDED_SHARED_SECRET_LENGTH];
    // SAFETY: the length of the provided okm is within the allowed range
    #[allow(clippy::unwrap_used)]
    hkdf.expand(EXPANDED_SHARED_SECRET_HKDF_INFO, &mut output)
        .unwrap();

    ExpandedSharedSecretWrapper(output)
}

/// Returns an all-zeroes [`Destination`] used as a placeholder for the final delivery address.
/// The sphinx protocol requires a destination, but for the agent's loopback packets the
/// address is irrelevant — the final hop (the agent itself) is already in the route.
fn dummy_destination() -> Destination {
    Destination::new(
        DestinationAddressBytes::from_bytes([0u8; DESTINATION_ADDRESS_LENGTH]),
        [0u8; IDENTIFIER_LENGTH],
    )
}

/// Builds a single test sphinx packet along `route` with the given per-hop `delay`.
///
/// The packet uses [`PacketSize::AckPacket`] to keep its size as small as possible. If `initial_secret`
/// is provided it is used as the sender's ephemeral key, allowing the resulting header to
/// be deterministically reproduced (needed for `create_test_sphinx_packet_header`).
pub(crate) fn build_test_sphinx_packet(
    route: &[Node; 2],
    delay: Duration,
    initial_secret: Option<&StaticSecret>,
    message: &[u8],
) -> anyhow::Result<SphinxPacket> {
    let delays = [
        Delay::new_from_nanos(delay.as_nanos() as u64),
        Delay::new_from_nanos(delay.as_nanos() as u64),
    ];
    let destination = dummy_destination();
    let payload = PacketSize::AckPacket.payload_size();

    let packet = match initial_secret {
        None => SphinxPacketBuilder::new()
            .with_payload_size(payload)
            .build_packet(message, route, &destination, &delays),
        Some(initial_secret) => SphinxPacketBuilder::new()
            .with_payload_size(payload)
            .with_initial_secret(initial_secret)
            .build_packet(message, route, &destination, &delays),
    }?;

    Ok(packet)
}

/// Builds a [`TestPacketHeader`] that can be reused to send many packets with different
/// payloads but the same routing header.
///
/// Internally this builds one full sphinx packet to capture the header, then manually
/// re-derives the per-hop payload keys by replaying the DH key-agreement steps along the
/// route. This is necessary because the sphinx library does not expose the payload keys
/// after packet construction.
///
/// The derived `payload_key` vec has one entry per hop; the last entry (index 1) is the
/// key held by this agent as the final recipient and is used by [`TestPacketHeader::recover_payload`].
pub(crate) fn create_test_sphinx_packet_header(
    route: [Node; 2],
    delay: Duration,
) -> anyhow::Result<TestPacketHeader> {
    let initial_secret = StaticSecret::random_from_rng(OsRng);

    // Build a throwaway packet solely to capture the reusable header.
    let packet = build_test_sphinx_packet(&route, delay, Some(&initial_secret), b"dummy-message")?;

    let header = packet.header;

    // Manually reconstruct the payload keys for each hop.
    let mut expanded_shared_secrets = Vec::new();
    let mut blinding_factors = Vec::new();

    // The sphinx library keeps these private, so we replicate the derivation:
    // for each hop, apply all previous blinding factors to the node's public key
    // via DH, then expand the result with HKDF to obtain the payload key.
    for node in &route {
        let mut acc = node.pub_key;

        for blinding_factor in std::iter::once(&initial_secret).chain(&blinding_factors) {
            let shared_secret = blinding_factor.diffie_hellman(&acc);
            acc = PublicKey::from(shared_secret.to_bytes());
        }

        let expanded_shared_secret = rederive_expanded_shared_secret(acc.as_bytes());
        blinding_factors.push(expanded_shared_secret.blinding_factor());
        expanded_shared_secrets.push(expanded_shared_secret);
    }

    let payload_keys = expanded_shared_secrets
        .iter()
        .map(|s| s.derive_payload_key())
        .collect::<Vec<_>>();
    assert_eq!(payload_keys.len(), 2);

    Ok(TestPacketHeader {
        header,
        payload_key: payload_keys,
    })
}

/// Constructs a sphinx [`Node`](Node) from a socket address and public key.
/// Panics if the address cannot be converted to a routing address, which should never happen
/// for a valid `SocketAddr`.
pub(crate) fn as_sphinx_node(address: SocketAddr, pub_key: x25519::PublicKey) -> Node {
    // SAFETY: we know that the address is valid, so we can safely unwrap it
    #[allow(clippy::unwrap_used)]
    Node::new(
        NymNodeRoutingAddress::from(address).try_into().unwrap(),
        pub_key.into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_packet::TestPacketContent;
    use nym_crypto::asymmetric::x25519;
    use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
    use nym_sphinx_types::ProcessedPacketData;
    use nym_test_utils::helpers::deterministic_rng;
    use std::net::SocketAddr;

    #[test]
    fn creating_test_sphinx_packets() {
        let mut rng = deterministic_rng();
        let remote_node_key = x25519::KeyPair::new(&mut rng);
        let agent_key = x25519::KeyPair::new(&mut rng);
        let node_addr: SocketAddr = "1.2.3.4:5677".parse().unwrap();
        let agent_addr: SocketAddr = "2.2.3.4:5678".parse().unwrap();

        let remote_node = Node::new(
            NymNodeRoutingAddress::from(node_addr).try_into().unwrap(),
            (*remote_node_key.public_key()).into(),
        );
        let agent_node = Node::new(
            NymNodeRoutingAddress::from(agent_addr).try_into().unwrap(),
            (*agent_key.public_key()).into(),
        );

        let delay = Duration::from_millis(1);

        let test_header =
            create_test_sphinx_packet_header([remote_node, agent_node], delay).unwrap();

        let payload1 = TestPacketContent::new(123);
        let payload2 = TestPacketContent::new(456);

        let packet1 = test_header.create_test_packet(payload1).unwrap();
        let packet2 = test_header.create_test_packet(payload2).unwrap();

        // simulate packet being received by remote node
        let res1 = packet1
            .process(remote_node_key.private_key().inner())
            .unwrap();
        let ProcessedPacketData::ForwardHop {
            next_hop_packet: res1_packet,
            next_hop_address,
            ..
        } = res1.data
        else {
            panic!("bad data")
        };
        assert_eq!(
            next_hop_address,
            NymNodeRoutingAddress::from(agent_addr).try_into().unwrap()
        );

        let res2 = packet2
            .process(remote_node_key.private_key().inner())
            .unwrap();
        let ProcessedPacketData::ForwardHop {
            next_hop_packet: res2_packet,
            next_hop_address,
            ..
        } = res2.data
        else {
            panic!("bad data")
        };
        assert_eq!(
            next_hop_address,
            NymNodeRoutingAddress::from(agent_addr).try_into().unwrap()
        );

        // now getting back to us (no need for full unwrapping as we already have the payload key)
        let received1 = test_header.recover_payload(res1_packet.payload).unwrap();
        assert_eq!(received1, payload1);

        let received2 = test_header.recover_payload(res2_packet.payload).unwrap();
        assert_eq!(received2, payload2);
    }
}

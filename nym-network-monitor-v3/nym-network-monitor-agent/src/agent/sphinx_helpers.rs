// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::agent::test_packet::TestPacketContent;
use anyhow::Context;
use arrayref::array_ref;
use hkdf::Hkdf;
use nym_crypto::aes::cipher::crypto_common::rand_core::OsRng;
use nym_sphinx_params::PacketSize;
use nym_sphinx_types::constants::{
    BLINDING_FACTOR_SIZE, EXPANDED_SHARED_SECRET_HKDF_INFO, EXPANDED_SHARED_SECRET_HKDF_SALT,
    EXPANDED_SHARED_SECRET_LENGTH, INTEGRITY_MAC_KEY_SIZE, PAYLOAD_KEY_SEED_SIZE,
};
use nym_sphinx_types::crypto::STREAM_CIPHER_KEY_SIZE;
use nym_sphinx_types::{
    DESTINATION_ADDRESS_LENGTH, Delay, Destination, DestinationAddressBytes, IDENTIFIER_LENGTH,
    Node, PAYLOAD_KEY_SIZE, Payload, PayloadKey, SphinxHeader, SphinxPacket, SphinxPacketBuilder,
    derive_payload_key,
};
use sha2::Sha256;
use std::time::Duration;
use x25519_dalek::{PublicKey, StaticSecret};

pub(crate) struct TestPacketHeader {
    pub(crate) header: SphinxHeader,
    pub(crate) payload_key: Vec<PayloadKey>,
}

impl TestPacketHeader {
    fn create_test_packet(&self, content: TestPacketContent) -> anyhow::Result<SphinxPacket> {
        let payload = Payload::encapsulate_message(
            &content.to_bytes(),
            &self.payload_key,
            PacketSize::AckPacket.payload_size(),
        )?;
        Ok(SphinxPacket {
            header: SphinxHeader {
                shared_secret: self.header.shared_secret,
                routing_info: self.header.routing_info.clone(),
            },
            payload,
        })
    }

    fn recover_payload(&self, received: Payload) -> anyhow::Result<TestPacketContent> {
        let key = self
            .payload_key
            .last()
            .context("no payload keys generated")?;

        let payload = received.unwrap(key)?.recover_plaintext()?;
        TestPacketContent::from_bytes(&payload)
    }
}

pub(crate) struct ExpandedSharedSecretWrapper(pub(crate) [u8; EXPANDED_SHARED_SECRET_LENGTH]);

impl ExpandedSharedSecretWrapper {
    pub(crate) fn blinding_factor(&self) -> StaticSecret {
        StaticSecret::from(*self.blinding_factor_bytes())
    }

    pub(crate) fn blinding_factor_bytes(&self) -> &[u8; BLINDING_FACTOR_SIZE] {
        array_ref!(
            &self.0,
            STREAM_CIPHER_KEY_SIZE + INTEGRITY_MAC_KEY_SIZE + PAYLOAD_KEY_SIZE,
            BLINDING_FACTOR_SIZE
        )
    }

    pub(crate) fn payload_key_seed(&self) -> &[u8; PAYLOAD_KEY_SEED_SIZE] {
        array_ref!(
            &self.0,
            STREAM_CIPHER_KEY_SIZE + INTEGRITY_MAC_KEY_SIZE,
            PAYLOAD_KEY_SEED_SIZE
        )
    }

    pub(crate) fn derive_payload_key(&self) -> PayloadKey {
        derive_payload_key(self.payload_key_seed())
    }
}

pub(crate) fn rederive_expanded_shared_secret(
    shared_secret: &[u8; 32],
) -> ExpandedSharedSecretWrapper {
    let hkdf = Hkdf::<Sha256>::new(Some(EXPANDED_SHARED_SECRET_HKDF_SALT), shared_secret);

    // expanded shared secret
    let mut output = [0u8; EXPANDED_SHARED_SECRET_LENGTH];
    // SAFETY: the length of the provided okm is within the allowed range
    #[allow(clippy::unwrap_used)]
    hkdf.expand(EXPANDED_SHARED_SECRET_HKDF_INFO, &mut output)
        .unwrap();

    ExpandedSharedSecretWrapper(output)
}

// if needed, we could use it for additional data
fn dummy_destination() -> Destination {
    Destination::new(
        DestinationAddressBytes::from_bytes([0u8; DESTINATION_ADDRESS_LENGTH]),
        [0u8; IDENTIFIER_LENGTH],
    )
}

pub(crate) fn create_test_sphinx_packet_header(
    route: Vec<Node>,
    delay: Duration,
) -> anyhow::Result<TestPacketHeader> {
    // we don't want any delays
    // and the packet route is test node -> this client
    let delays = [
        Delay::new_from_nanos(delay.as_nanos() as u64),
        Delay::new_from_nanos(delay.as_nanos() as u64),
    ];
    let destination = dummy_destination();
    // we use acks for their reduced size
    let payload = PacketSize::AckPacket.payload_size();

    let initial_secret = StaticSecret::random_from_rng(OsRng);
    let packet = SphinxPacketBuilder::new()
        .with_payload_size(payload)
        .with_initial_secret(&initial_secret)
        .build_packet("dummy-content", &route, &destination, &delays)?;

    let header = packet.header;

    // rebuild the payload keys (unfortunately, we can't use existing methods as they are not public)
    let mut expanded_shared_secrets = Vec::new();
    let mut blinding_factors = Vec::new();

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

#[cfg(test)]
mod tests {
    use super::*;
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
            create_test_sphinx_packet_header(vec![remote_node, agent_node], delay).unwrap();

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

// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::mixnet::sphinx::test_packet::TestPacketHeader;
use anyhow::bail;
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
    Delay, Destination, DestinationAddressBytes, IDENTIFIER_LENGTH, MAX_PATH_LENGTH, Node,
    PAYLOAD_KEY_SIZE, PayloadKey, SphinxPacket, SphinxPacketBuilder, derive_payload_key,
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

/// The sphinx destination for a payload meant for `client_address`.
///
/// The SURB identifier is zeroed: it exists for reply SURBs and a test packet carries none. A node's
/// final-hop processing yields only the ADDRESS, so nothing downstream reads it.
fn test_destination(client_address: DestinationAddressBytes) -> Destination {
    Destination::new(client_address, [0u8; IDENTIFIER_LENGTH])
}

/// Rejects a route sphinx cannot build a packet along.
///
/// Both ends matter now that the length is a runtime value rather than a fixed-size array. Sphinx
/// PANICS at either bound rather than reporting them: an empty route underflows `route.len() - 1`
/// while constructing the header filler, and one longer than [`MAX_PATH_LENGTH`] trips an assert
/// inside it. A probe builds its route from an assignment, so a bad one has to surface as a failed
/// measurement rather than as a dead agent.
fn validate_route(route: &[Node]) -> anyhow::Result<()> {
    if route.is_empty() {
        bail!("attempted to build a test packet along an empty route")
    }
    if route.len() > MAX_PATH_LENGTH {
        bail!(
            "attempted to build a test packet along a route of {} hops, which exceeds the sphinx maximum of {MAX_PATH_LENGTH}",
            route.len()
        )
    }
    Ok(())
}

/// Builds a single test sphinx packet along `route` with the given per-hop `delay`, addressed to
/// `client_address`.
///
/// `route` is any length sphinx accepts. The mixnode probe uses two hops (the node, then this agent),
/// while each leg of the gateway probe uses one, since there the tested node and the packet's last
/// hop are the same machine.
///
/// `client_address` is always THIS agent's own client address. Whether anything reads it depends on
/// the last hop: a gateway that is asked to deliver a final-hop packet resolves one of its live
/// sessions by it, so a wrong value there is silently dropped and reads as a dead node, whereas on a
/// route whose last hop is the agent itself nobody ever looks at it. Passing the real address in both
/// cases costs nothing and leaves no placeholder to get wrong.
///
/// The packet uses [`PacketSize::AckPacket`] to keep its size as small as possible. If `initial_secret`
/// is provided it is used as the sender's ephemeral key, allowing the resulting header to
/// be deterministically reproduced (needed for `create_test_sphinx_packet_header`).
pub(crate) fn build_test_sphinx_packet(
    route: &[Node],
    client_address: DestinationAddressBytes,
    delay: Duration,
    initial_secret: Option<&StaticSecret>,
    message: &[u8],
) -> anyhow::Result<SphinxPacket> {
    validate_route(route)?;

    // one delay per hop: sphinx pairs them off positionally, so a mismatched count silently
    // misassigns them
    let delays = vec![Delay::new_from_nanos(delay.as_nanos() as u64); route.len()];
    let destination = test_destination(client_address);
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
/// The derived `payload_key` vec has one entry per hop; the LAST entry is the key held by this agent
/// as the final recipient and is the one [`TestPacketHeader::recover_payload`] unwraps with. That is
/// why it is indexed from the end rather than at a fixed position: a one-hop route has a single key
/// which is also the final one.
/// The destination is baked into the captured header's routing info, so it is supplied ONCE here
/// rather than per packet: every packet [`TestPacketHeader::create_test_packet`] goes on to build
/// carries the same one.
pub(crate) fn create_test_sphinx_packet_header(
    route: &[Node],
    client_address: DestinationAddressBytes,
    delay: Duration,
) -> anyhow::Result<TestPacketHeader> {
    let initial_secret = StaticSecret::random_from_rng(OsRng);

    // Build a throwaway packet solely to capture the reusable header. Validates the route too, so
    // the derivation below cannot run against one sphinx would have rejected.
    let packet = build_test_sphinx_packet(
        route,
        client_address,
        delay,
        Some(&initial_secret),
        b"dummy-message",
    )?;

    let header = packet.header;

    // Manually reconstruct the payload keys for each hop.
    let mut expanded_shared_secrets = Vec::with_capacity(route.len());
    let mut blinding_factors = Vec::with_capacity(route.len());

    // The sphinx library keeps these private, so we replicate the derivation:
    // for each hop, apply all previous blinding factors to the node's public key
    // via DH, then expand the result with HKDF to obtain the payload key.
    for node in route {
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
    use crate::mixnet::sphinx::test_packet::TestPacketContent;
    use nym_crypto::asymmetric::x25519;
    use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
    use nym_sphinx_types::{DESTINATION_ADDRESS_LENGTH, ProcessedPacketData};
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
            create_test_sphinx_packet_header(&[remote_node, agent_node], client_address(), delay)
                .unwrap();

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

    fn node(address: &str, key: &x25519::KeyPair) -> Node {
        as_sphinx_node(
            address.parse().expect("malformed test address"),
            *key.public_key(),
        )
    }

    /// A stand-in for the agent's own client address. Distinctive rather than zeroed, so a test
    /// asserting on it cannot pass against a placeholder.
    fn client_address() -> DestinationAddressBytes {
        DestinationAddressBytes::from_bytes([9u8; DESTINATION_ADDRESS_LENGTH])
    }

    // the whole point of carrying a destination: a gateway asked to deliver a final-hop packet
    // resolves one of its live sessions by exactly this address, so it has to survive unwrapping
    #[test]
    fn the_client_address_reaches_the_final_hop_intact() {
        let mut rng = deterministic_rng();
        let gateway_key = x25519::KeyPair::new(&mut rng);
        let route = [node("1.2.3.4:1789", &gateway_key)];

        let packet = build_test_sphinx_packet(
            &route,
            client_address(),
            Duration::from_millis(1),
            None,
            &TestPacketContent::new(1).to_bytes(),
        )
        .unwrap();

        let processed = packet.process(gateway_key.private_key().inner()).unwrap();
        let ProcessedPacketData::FinalHop { destination, .. } = processed.data else {
            panic!("a one hop route did not terminate at its only hop")
        };
        assert_eq!(destination, client_address());
    }

    // the reusable header bakes the destination into its routing info, so every packet built from one
    // header carries the same address without it being supplied again
    #[test]
    fn a_reusable_header_carries_its_client_address_into_every_packet() {
        let mut rng = deterministic_rng();
        let gateway_key = x25519::KeyPair::new(&mut rng);
        let route = [node("1.2.3.4:1789", &gateway_key)];

        let header =
            create_test_sphinx_packet_header(&route, client_address(), Duration::from_millis(1))
                .unwrap();

        for id in [1, 2] {
            let packet = header
                .create_test_packet(TestPacketContent::new(id))
                .unwrap();
            let processed = packet.process(gateway_key.private_key().inner()).unwrap();
            let ProcessedPacketData::FinalHop { destination, .. } = processed.data else {
                panic!("a one hop route did not terminate at its only hop")
            };
            assert_eq!(destination, client_address(), "packet {id}");
        }
    }

    // A gateway probe's legs are ONE hop, because the tested node and the packet's final hop are the
    // same machine. Nothing in the sphinx layer forbids it, but nothing exercised it either while
    // both helpers were fixed at two.
    #[test]
    fn a_one_hop_route_produces_a_packet_the_single_hop_can_open() {
        let mut rng = deterministic_rng();
        let gateway_key = x25519::KeyPair::new(&mut rng);
        let route = [node("1.2.3.4:1789", &gateway_key)];

        let content = TestPacketContent::new(7);
        let packet = build_test_sphinx_packet(
            &route,
            client_address(),
            Duration::from_millis(1),
            None,
            &content.to_bytes(),
        )
        .expect("a single hop route was rejected");

        // the one hop IS the final hop, so processing yields a payload rather than a forward
        let processed = packet
            .process(gateway_key.private_key().inner())
            .expect("the single hop could not process the packet");
        let ProcessedPacketData::FinalHop { payload, .. } = processed.data else {
            panic!("a one hop route did not terminate at its only hop")
        };
        assert_eq!(
            TestPacketContent::from_bytes(&payload.recover_plaintext().unwrap()).unwrap(),
            content
        );
    }

    // the reusable header has to work at one hop too, and its payload key is taken from the END of
    // the derived list, so a single-hop route's only key is also its final one
    #[test]
    fn a_one_hop_reusable_header_derives_exactly_one_payload_key() {
        let mut rng = deterministic_rng();
        let gateway_key = x25519::KeyPair::new(&mut rng);
        let route = [node("1.2.3.4:1789", &gateway_key)];

        let header =
            create_test_sphinx_packet_header(&route, client_address(), Duration::from_millis(1))
                .expect("a single hop route was rejected");
        assert_eq!(header.payload_key.len(), 1);

        let content = TestPacketContent::new(11);
        let packet = header.create_test_packet(content).unwrap();

        let processed = packet.process(gateway_key.private_key().inner()).unwrap();
        let ProcessedPacketData::FinalHop { payload, .. } = processed.data else {
            panic!("a one hop route did not terminate at its only hop")
        };
        assert_eq!(
            TestPacketContent::from_bytes(&payload.recover_plaintext().unwrap()).unwrap(),
            content
        );
    }

    // a route is built from an assignment, so a bad one has to fail the measurement rather than the
    // agent. sphinx underflows `route.len() - 1` building the header filler
    #[test]
    fn an_empty_route_is_refused_rather_than_panicking() {
        // `SphinxPacket` has no `Debug`, so the error is taken by matching rather than `expect_err`
        let Err(err) = build_test_sphinx_packet(
            &[],
            client_address(),
            Duration::from_millis(1),
            None,
            b"payload",
        ) else {
            panic!("an empty route was accepted")
        };
        assert!(err.to_string().contains("empty route"), "{err}");

        assert!(
            create_test_sphinx_packet_header(&[], client_address(), Duration::from_millis(1))
                .is_err()
        );
    }

    // the other bound sphinx enforces with an assert rather than an error
    #[test]
    fn a_route_longer_than_sphinx_allows_is_refused() {
        let mut rng = deterministic_rng();
        let route = (0..MAX_PATH_LENGTH + 1)
            .map(|i| {
                let key = x25519::KeyPair::new(&mut rng);
                node(&format!("1.2.3.4:{}", 1789 + i), &key)
            })
            .collect::<Vec<_>>();

        let Err(err) = build_test_sphinx_packet(
            &route,
            client_address(),
            Duration::from_millis(1),
            None,
            b"payload",
        ) else {
            panic!("an over-long route was accepted")
        };
        assert!(
            err.to_string().contains("exceeds the sphinx maximum"),
            "{err}"
        );
    }

    // one delay per hop, since sphinx pairs them positionally and a mismatch would misassign them
    #[test]
    fn every_hop_of_a_route_is_given_the_configured_delay() {
        let mut rng = deterministic_rng();
        let first_key = x25519::KeyPair::new(&mut rng);
        let second_key = x25519::KeyPair::new(&mut rng);
        let route = [
            node("1.2.3.4:1789", &first_key),
            node("2.3.4.5:1789", &second_key),
        ];
        let delay = Duration::from_millis(50);

        let packet = build_test_sphinx_packet(
            &route,
            client_address(),
            delay,
            None,
            &TestPacketContent::new(1).to_bytes(),
        )
        .unwrap();

        let first = packet.process(first_key.private_key().inner()).unwrap();
        let ProcessedPacketData::ForwardHop {
            next_hop_packet,
            delay: first_delay,
            ..
        } = first.data
        else {
            panic!("the first of two hops was not a forward hop")
        };
        assert_eq!(first_delay.to_nanos(), delay.as_nanos() as u64);

        let second = next_hop_packet
            .process(second_key.private_key().inner())
            .unwrap();
        let ProcessedPacketData::FinalHop { .. } = second.data else {
            panic!("the second of two hops was not the final hop")
        };
    }
}

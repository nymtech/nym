// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::sphinx_helpers::as_sphinx_node;
use nym_crypto::asymmetric::x25519;
use nym_network_monitor_orchestrator_requests::models::MixnetProbeTarget;
use nym_noise::config::{NoiseNode, NoiseVersion, VersionedNoiseKeyV1};
use nym_sphinx_params::SphinxKeyRotation;
use std::net::{IpAddr, SocketAddr};

/// Identity and addressing information for the node being tested in a stress-test run.
#[derive(Debug, Clone)]
pub(crate) struct TestedNodeDetails {
    pub(crate) node_id: Option<u32>,

    /// TCP socket address of the node's mixnet listener, used for the egress connection.
    pub(crate) address: SocketAddr,

    /// Every ip address the node is known by, canonicalised. The node may return the test packets
    /// from an address other than the one it was reached on (it may be multi-homed, or be replying
    /// over a different family), so all of them have to be treated as the node itself.
    pub(crate) known_ips: Vec<IpAddr>,

    /// Node's static Noise public key, used to authenticate and encrypt the egress connection.
    pub(crate) noise_key: x25519::PublicKey,

    /// Key rotation associated with the current sphinx key of the node.
    pub(crate) key_rotation: SphinxKeyRotation,

    /// Node's current sphinx public key, used to build the sphinx packet header.
    pub(crate) sphinx_key: x25519::PublicKey,
}

impl TestedNodeDetails {
    pub(crate) fn from_probe_target(target: MixnetProbeTarget) -> Self {
        // the assigned address is always one of the announced ones, but make sure it's in the set
        // regardless: everything downstream treats this as "the addresses that are this node"
        let mut known_ips = target
            .node_ips
            .iter()
            .chain(std::iter::once(&target.node_address.ip()))
            .map(|ip| ip.to_canonical())
            .collect::<Vec<_>>();
        known_ips.sort_unstable();
        known_ips.dedup();

        TestedNodeDetails {
            node_id: Some(target.node_id),
            address: target.node_address,
            known_ips,
            noise_key: target.noise_key,
            key_rotation: SphinxKeyRotation::from_key_rotation_id(target.key_rotation_id),
            sphinx_key: target.sphinx_key,
        }
    }

    /// Returns a sphinx [`Node`](nym_sphinx_types::Node) representation of this node,
    /// suitable for use as a hop in a sphinx route.
    pub(crate) fn as_sphinx_node(&self) -> nym_sphinx_types::Node {
        as_sphinx_node(self.address, self.sphinx_key)
    }

    /// Returns a [`NoiseNode`] representation of this node for use in the Noise network view.
    pub(crate) fn as_noise_node(&self) -> NoiseNode {
        NoiseNode::new_nym_node(VersionedNoiseKeyV1 {
            supported_version: NoiseVersion::V1,
            x25519_pubkey: self.noise_key,
        })
    }
}

#[cfg(test)]
impl TestedNodeDetails {
    /// A node reachable at `address` and known by `known_ips`, with throwaway keys. The key values
    /// are irrelevant to every caller; only their distinctness between nodes matters.
    pub(crate) fn new_test(address: SocketAddr, known_ips: &[IpAddr]) -> Self {
        let mut rng = rand::rngs::OsRng;
        let noise_key = x25519::PublicKey::from(&x25519::PrivateKey::new(&mut rng));
        let sphinx_key = x25519::PublicKey::from(&x25519::PrivateKey::new(&mut rng));

        TestedNodeDetails {
            node_id: Some(1),
            address,
            // canonicalised for the same reason `from_probe_target` does it: everything downstream
            // treats this as "the addresses that are this node"
            known_ips: known_ips.iter().map(|ip| ip.to_canonical()).collect(),
            noise_key,
            key_rotation: SphinxKeyRotation::from_key_rotation_id(0),
            sphinx_key,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_test_utils::helpers::deterministic_rng;

    fn target(node_address: &str, node_ips: &[&str]) -> MixnetProbeTarget {
        let mut rng = deterministic_rng();
        let key = x25519::PublicKey::from(&x25519::PrivateKey::new(&mut rng));
        MixnetProbeTarget {
            node_id: 42,
            identity_key: *nym_crypto::asymmetric::ed25519::KeyPair::new(&mut rng).public_key(),
            node_address: node_address.parse().unwrap(),
            node_ips: node_ips.iter().map(|ip| ip.parse().unwrap()).collect(),
            noise_key: key,
            sphinx_key: key,
            key_rotation_id: 0,
        }
    }

    fn ip(raw: &str) -> IpAddr {
        raw.parse().expect("malformed test ip")
    }

    // a node may be multi-homed, or be reached over one family and reply over another, so EVERY
    // announced address is retained: the wave's ingress attributes a return connection from any of
    // them to this node
    #[test]
    fn every_announced_address_is_retained() {
        let node = TestedNodeDetails::from_probe_target(target(
            "[aaaa::1]:1789",
            &["1.1.1.1", "2.2.2.2", "aaaa::1"],
        ));

        assert!(node.known_ips.contains(&ip("1.1.1.1")));
        assert!(node.known_ips.contains(&ip("2.2.2.2")));
        assert!(node.known_ips.contains(&ip("aaaa::1")));
        assert!(!node.known_ips.contains(&ip("9.9.9.9")));
    }

    // stored canonically, so that a v4-mapped announcement and the v4 form it denotes are one entry
    // rather than two that fail to match a peer address
    #[test]
    fn an_ipv4_mapped_announcement_is_stored_canonically() {
        let node =
            TestedNodeDetails::from_probe_target(target("1.1.1.1:1789", &["::ffff:1.1.1.1"]));

        assert_eq!(node.known_ips, vec![ip("1.1.1.1")]);
    }

    // the assigned address is always one of the announced ones, but the set has to hold regardless
    #[test]
    fn the_tested_address_is_always_retained() {
        let node = TestedNodeDetails::from_probe_target(target("3.3.3.3:1789", &[]));

        assert_eq!(node.known_ips, vec![ip("3.3.3.3")]);
    }
}

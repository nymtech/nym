// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::sphinx_helpers::as_sphinx_node;
use nym_crypto::asymmetric::x25519;
use nym_network_monitor_orchestrator_requests::models::TestRunAssignment;
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
    pub(crate) fn from_testrun_assignment(assignment: TestRunAssignment) -> Self {
        // the assigned address is always one of the announced ones, but make sure it's in the set
        // regardless: everything downstream treats this as "the addresses that are this node"
        let mut known_ips = assignment
            .node_ips
            .iter()
            .chain(std::iter::once(&assignment.node_address.ip()))
            .map(|ip| ip.to_canonical())
            .collect::<Vec<_>>();
        known_ips.sort_unstable();
        known_ips.dedup();

        TestedNodeDetails {
            node_id: Some(assignment.node_id),
            address: assignment.node_address,
            known_ips,
            noise_key: assignment.noise_key,
            key_rotation: SphinxKeyRotation::from_key_rotation_id(assignment.key_rotation_id),
            sphinx_key: assignment.sphinx_key,
        }
    }

    /// Whether `source` is one of the addresses this node is known by, i.e. whether an inbound
    /// connection from it can be the node returning our test packets. Canonicalised because a
    /// dual-stack listener reports ipv4 peers in their ipv4-mapped ipv6 form.
    pub(crate) fn is_known_source(&self, source: IpAddr) -> bool {
        self.known_ips.contains(&source.to_canonical())
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
mod tests {
    use super::*;
    use nym_test_utils::helpers::deterministic_rng;

    fn assignment(node_address: &str, node_ips: &[&str]) -> TestRunAssignment {
        let key = x25519::PublicKey::from(&x25519::PrivateKey::new(&mut deterministic_rng()));
        TestRunAssignment {
            node_id: 42,
            node_address: node_address.parse().unwrap(),
            node_ips: node_ips.iter().map(|ip| ip.parse().unwrap()).collect(),
            noise_key: key,
            sphinx_key: key,
            key_rotation_id: 0,
        }
    }

    // a node may be multi-homed, or be reached over one family and reply over another, so the
    // return connection has to be accepted from any address the orchestrator knows it by
    #[test]
    fn any_announced_address_is_a_known_source() {
        let node = TestedNodeDetails::from_testrun_assignment(assignment(
            "[aaaa::1]:1789",
            &["1.1.1.1", "2.2.2.2", "aaaa::1"],
        ));

        assert!(node.is_known_source("1.1.1.1".parse().unwrap()));
        assert!(node.is_known_source("2.2.2.2".parse().unwrap()));
        assert!(node.is_known_source("aaaa::1".parse().unwrap()));
        assert!(!node.is_known_source("9.9.9.9".parse().unwrap()));
    }

    // a dual-stack listener reports ipv4 peers in their ipv4-mapped form
    #[test]
    fn an_ipv4_mapped_source_matches_its_canonical_form() {
        let node =
            TestedNodeDetails::from_testrun_assignment(assignment("1.1.1.1:1789", &["1.1.1.1"]));

        assert!(node.is_known_source("::ffff:1.1.1.1".parse().unwrap()));
    }

    // the assigned address is always announced, but the set has to hold regardless
    #[test]
    fn the_tested_address_is_always_a_known_source() {
        let node = TestedNodeDetails::from_testrun_assignment(assignment("3.3.3.3:1789", &[]));

        assert!(node.is_known_source("3.3.3.3".parse().unwrap()));
    }
}

// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::sphinx_helpers::as_sphinx_node;
use nym_crypto::asymmetric::x25519;
use nym_noise::config::{NoiseNode, NoiseVersion, VersionedNoiseKeyV1};
use nym_sphinx_params::SphinxKeyRotation;
use std::net::SocketAddr;

/// Identity and addressing information for the node being tested in a stress-test run.
#[derive(Debug)]
pub(crate) struct TestedNodeDetails {
    /// TCP socket address of the node's mixnet listener, used for the egress connection.
    pub(crate) address: SocketAddr,

    /// Node's static Noise public key, used to authenticate and encrypt the egress connection.
    pub(crate) noise_key: x25519::PublicKey,

    /// Key rotation associated with the current sphinx key of the node.
    pub(crate) key_rotation: SphinxKeyRotation,

    /// Node's current sphinx public key, used to build the sphinx packet header.
    pub(crate) sphinx_key: x25519::PublicKey,
}

impl TestedNodeDetails {
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

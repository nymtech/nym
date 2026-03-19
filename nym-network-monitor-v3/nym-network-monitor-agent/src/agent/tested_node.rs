// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_crypto::asymmetric::x25519;
use nym_noise::config::NoiseNode;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_params::SphinxKeyRotation;
use std::net::SocketAddr;

pub(crate) struct TestedNodeDetails {
    pub(crate) address: SocketAddr,

    pub(crate) noise_key: x25519::PublicKey,

    /// Key rotation associated with the current sphinx key of the node
    pub(crate) key_rotation: SphinxKeyRotation,

    pub(crate) sphinx_key: x25519::PublicKey,
}

impl TestedNodeDetails {
    pub(crate) fn as_sphinx_node(&self) -> anyhow::Result<nym_sphinx_types::Node> {
        Ok(nym_sphinx_types::Node::new(
            NymNodeRoutingAddress::from(self.address).try_into()?,
            self.sphinx_key.into(),
        ))
    }

    pub(crate) fn as_noise_node(&self) -> NoiseNode {
        NoiseNode::new_from_inner_key(self.noise_key, 1, true)
    }
}

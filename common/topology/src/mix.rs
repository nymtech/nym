// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{filter, NetworkAddress};
use nym_crypto::asymmetric::{encryption, identity};
pub use nym_mixnet_contract_common::Layer;
use nym_mixnet_contract_common::{MixId, MixNodeBond};
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_types::Node as SphinxNode;
use std::convert::{TryFrom, TryInto};
use std::io;
use std::net::SocketAddr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MixnodeConversionError {
    #[error("mixnode identity key was malformed - {0}")]
    InvalidIdentityKey(#[from] identity::Ed25519RecoveryError),

    #[error("mixnode sphinx key was malformed - {0}")]
    InvalidSphinxKey(#[from] encryption::KeyRecoveryError),

    #[error("'{value}' is not a valid mixnode address - {source}")]
    InvalidAddress {
        value: String,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, Clone)]
pub struct Node {
    pub mix_id: MixId,
    pub owner: String,
    pub host: NetworkAddress,
    // we're keeping this as separate resolved field since we do not want to be resolving the potential
    // hostname every time we want to construct a path via this node
    pub mix_host: SocketAddr,
    pub identity_key: identity::PublicKey,
    pub sphinx_key: encryption::PublicKey, // TODO: or nymsphinx::PublicKey? both are x25519
    pub layer: Layer,
    pub version: String,
}

impl filter::Versioned for Node {
    fn version(&self) -> String {
        self.version.clone()
    }
}

impl<'a> From<&'a Node> for SphinxNode {
    fn from(node: &'a Node) -> Self {
        let node_address_bytes = NymNodeRoutingAddress::from(node.mix_host)
            .try_into()
            .unwrap();

        SphinxNode::new(node_address_bytes, (&node.sphinx_key).into())
    }
}

impl<'a> TryFrom<&'a MixNodeBond> for Node {
    type Error = MixnodeConversionError;

    fn try_from(bond: &'a MixNodeBond) -> Result<Self, Self::Error> {
        let host: NetworkAddress =
            bond.mix_node
                .host
                .parse()
                .map_err(|err| MixnodeConversionError::InvalidAddress {
                    value: bond.mix_node.host.clone(),
                    source: err,
                })?;

        // try to completely resolve the host in the mix situation to avoid doing it every
        // single time we want to construct a path
        let mix_host = host
            .to_socket_addrs(bond.mix_node.mix_port)
            .map_err(|err| MixnodeConversionError::InvalidAddress {
                value: bond.mix_node.host.clone(),
                source: err,
            })?[0];

        Ok(Node {
            mix_id: bond.mix_id,
            owner: bond.owner.as_str().to_owned(),
            host,
            mix_host,
            identity_key: identity::PublicKey::from_base58_string(&bond.mix_node.identity_key)?,
            sphinx_key: encryption::PublicKey::from_base58_string(&bond.mix_node.sphinx_key)?,
            layer: bond.layer,
            version: bond.mix_node.version.clone(),
        })
    }
}

impl TryFrom<MixNodeBond> for Node {
    type Error = MixnodeConversionError;

    fn try_from(bond: MixNodeBond) -> Result<Self, Self::Error> {
        Node::try_from(&bond)
    }
}

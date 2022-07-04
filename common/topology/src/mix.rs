// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{filter, NetworkAddress};
use crypto::asymmetric::{encryption, identity};
use mixnet_contract_common::{Layer, MixNodeBond};
use nymsphinx_addressing::nodes::NymNodeRoutingAddress;
use nymsphinx_types::Node as SphinxNode;
use std::convert::{TryFrom, TryInto};
use std::fmt::{self, Display, Formatter};
use std::io;
use std::net::SocketAddr;

#[derive(Debug)]
pub enum MixnodeConversionError {
    InvalidIdentityKey(identity::Ed25519RecoveryError),
    InvalidSphinxKey(encryption::KeyRecoveryError),
    InvalidAddress(String, io::Error),
    InvalidStake,
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl From<encryption::KeyRecoveryError> for MixnodeConversionError {
    fn from(err: encryption::KeyRecoveryError) -> Self {
        MixnodeConversionError::InvalidSphinxKey(err)
    }
}

impl From<identity::Ed25519RecoveryError> for MixnodeConversionError {
    fn from(err: identity::Ed25519RecoveryError) -> Self {
        MixnodeConversionError::InvalidIdentityKey(err)
    }
}

impl Display for MixnodeConversionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            MixnodeConversionError::InvalidIdentityKey(err) => write!(
                f,
                "failed to convert mixnode due to invalid identity key - {}",
                err
            ),
            MixnodeConversionError::InvalidSphinxKey(err) => write!(
                f,
                "failed to convert mixnode due to invalid sphinx key - {}",
                err
            ),
            MixnodeConversionError::InvalidAddress(address, err) => {
                write!(
                    f,
                    "failed to convert mixnode due to invalid address {} - {}",
                    address, err
                )
            }
            MixnodeConversionError::InvalidStake => {
                write!(f, "failed to convert mixnode due to invalid stake")
            }
            MixnodeConversionError::Other(err) => {
                write!(
                    f,
                    "failed to convert mixnode due to another error - {}",
                    err
                )
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub owner: String,
    // somebody correct me if I'm wrong, but we should only ever have a single denom of currency
    // on the network at a type, right?
    pub stake: u128,
    pub delegation: u128,
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
        let host: NetworkAddress = bond.mix_node.host.parse().map_err(|err| {
            MixnodeConversionError::InvalidAddress(bond.mix_node.host.clone(), err)
        })?;

        // try to completely resolve the host in the mix situation to avoid doing it every
        // single time we want to construct a path
        let mix_host = host
            .to_socket_addrs(bond.mix_node.mix_port)
            .map_err(|err| {
                MixnodeConversionError::InvalidAddress(bond.mix_node.host.clone(), err)
            })?[0];

        Ok(Node {
            owner: bond.owner.as_str().to_owned(),
            stake: bond.original_pledge.amount.into(),
            delegation: bond.total_delegation.amount.into(),
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

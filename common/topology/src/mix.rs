// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::filter;
use crypto::asymmetric::{encryption, identity};
use mixnet_contract::MixNodeBond;
use nymsphinx_addressing::nodes::NymNodeRoutingAddress;
use nymsphinx_types::Node as SphinxNode;
use std::convert::{TryFrom, TryInto};
use std::fmt::{self, Display, Formatter};
use std::io;
use std::net::SocketAddr;

#[derive(Debug)]
pub enum MixnodeConversionError {
    InvalidIdentityKey(identity::KeyRecoveryError),
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

impl From<identity::KeyRecoveryError> for MixnodeConversionError {
    fn from(err: identity::KeyRecoveryError) -> Self {
        MixnodeConversionError::InvalidIdentityKey(err)
    }
}

impl Display for MixnodeConversionError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
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
    pub location: String,
    pub host: SocketAddr,
    pub identity_key: identity::PublicKey,
    pub sphinx_key: encryption::PublicKey, // TODO: or nymsphinx::PublicKey? both are x25519
    pub layer: u64,
    pub version: String,
}

impl filter::Versioned for Node {
    fn version(&self) -> String {
        self.version.clone()
    }
}

impl<'a> From<&'a Node> for SphinxNode {
    fn from(node: &'a Node) -> Self {
        let node_address_bytes = NymNodeRoutingAddress::from(node.host).try_into().unwrap();

        SphinxNode::new(node_address_bytes, (&node.sphinx_key).into())
    }
}

impl<'a> TryFrom<&'a MixNodeBond> for Node {
    type Error = MixnodeConversionError;

    fn try_from(bond: &'a MixNodeBond) -> Result<Self, Self::Error> {
        if bond.amount.len() > 1 {
            return Err(MixnodeConversionError::InvalidStake);
        }
        Ok(Node {
            owner: bond.owner.0.clone(),
            stake: bond
                .amount
                .first()
                .map(|stake| stake.amount.into())
                .unwrap_or(0),
            location: bond.mix_node.location.clone(),
            host: bond.mix_node.try_resolve_hostname().map_err(|err| {
                MixnodeConversionError::InvalidAddress(bond.mix_node.host.clone(), err)
            })?,
            identity_key: identity::PublicKey::from_base58_string(&bond.mix_node.identity_key)?,
            sphinx_key: encryption::PublicKey::from_base58_string(&bond.mix_node.sphinx_key)?,
            layer: bond.mix_node.layer,
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

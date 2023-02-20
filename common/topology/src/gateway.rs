// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{filter, NetworkAddress};
use nym_crypto::asymmetric::{encryption, identity};
use nym_mixnet_contract_common::GatewayBond;
use nym_sphinx_addressing::nodes::{NodeIdentity, NymNodeRoutingAddress};
use nym_sphinx_types::Node as SphinxNode;
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::io;
use std::net::SocketAddr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GatewayConversionError {
    #[error("gateway identity key was malformed - {0}")]
    InvalidIdentityKey(#[from] identity::Ed25519RecoveryError),

    #[error("gateway sphinx key was malformed - {0}")]
    InvalidSphinxKey(#[from] encryption::KeyRecoveryError),

    #[error("'{value}' is not a valid gateway address - {source}")]
    InvalidAddress {
        value: String,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, Clone)]
pub struct Node {
    pub owner: String,
    // somebody correct me if I'm wrong, but we should only ever have a single denom of currency
    // on the network at a type, right?
    pub stake: u128,
    pub location: String,
    pub host: NetworkAddress,
    // we're keeping this as separate resolved field since we do not want to be resolving the potential
    // hostname every time we want to construct a path via this node
    pub mix_host: SocketAddr,
    pub clients_port: u16,
    pub identity_key: identity::PublicKey,
    pub sphinx_key: encryption::PublicKey, // TODO: or nymsphinx::PublicKey? both are x25519
    pub version: String,
}

impl Node {
    pub fn identity(&self) -> &NodeIdentity {
        &self.identity_key
    }

    pub fn clients_address(&self) -> String {
        format!("ws://{}:{}", self.host, self.clients_port)
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Node(id: {}, owner: {}, stake: {}, location: {}, host: {})",
            self.identity_key, self.owner, self.stake, self.location, self.host,
        )
    }
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

impl<'a> TryFrom<&'a GatewayBond> for Node {
    type Error = GatewayConversionError;

    fn try_from(bond: &'a GatewayBond) -> Result<Self, Self::Error> {
        let host: NetworkAddress =
            bond.gateway
                .host
                .parse()
                .map_err(|err| GatewayConversionError::InvalidAddress {
                    value: bond.gateway.host.clone(),
                    source: err,
                })?;

        // try to completely resolve the host in the mix situation to avoid doing it every
        // single time we want to construct a path
        let mix_host = host.to_socket_addrs(bond.gateway.mix_port).map_err(|err| {
            GatewayConversionError::InvalidAddress {
                value: bond.gateway.host.clone(),
                source: err,
            }
        })?[0];

        Ok(Node {
            owner: bond.owner.as_str().to_owned(),
            stake: bond.pledge_amount.amount.into(),
            location: bond.gateway.location.clone(),
            host,
            mix_host,
            clients_port: bond.gateway.clients_port,
            identity_key: identity::PublicKey::from_base58_string(&bond.gateway.identity_key)?,
            sphinx_key: encryption::PublicKey::from_base58_string(&bond.gateway.sphinx_key)?,
            version: bond.gateway.version.clone(),
        })
    }
}

impl TryFrom<GatewayBond> for Node {
    type Error = GatewayConversionError;

    fn try_from(bond: GatewayBond) -> Result<Self, Self::Error> {
        Node::try_from(&bond)
    }
}

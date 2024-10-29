// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{filter, NetworkAddress, NodeVersion};
use nym_api_requests::nym_nodes::{NodeRole, SkimmedNode};
use nym_crypto::asymmetric::{encryption, identity};
pub use nym_mixnet_contract_common::LegacyMixLayer;
use nym_mixnet_contract_common::NodeId;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_types::Node as SphinxNode;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fmt::Formatter;
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

    #[error("invalid mix layer")]
    InvalidLayer,

    #[error("'{mixnode}' has not provided any valid ip addresses")]
    NoIpAddressesProvided { mixnode: String },

    #[error("provided node is not a mixnode in this epoch!")]
    NotMixnode,
}

#[derive(Clone)]
pub struct LegacyNode {
    pub mix_id: NodeId,
    pub host: NetworkAddress,
    // we're keeping this as separate resolved field since we do not want to be resolving the potential
    // hostname every time we want to construct a path via this node
    pub mix_host: SocketAddr,
    pub identity_key: identity::PublicKey,
    pub sphinx_key: encryption::PublicKey, // TODO: or nymsphinx::PublicKey? both are x25519
    pub layer: LegacyMixLayer,

    // to be removed:
    pub version: NodeVersion,
}

impl std::fmt::Debug for LegacyNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("mix::Node")
            .field("mix_id", &self.mix_id)
            .field("host", &self.host)
            .field("mix_host", &self.mix_host)
            .field("identity_key", &self.identity_key.to_base58_string())
            .field("sphinx_key", &self.sphinx_key.to_base58_string())
            .field("layer", &self.layer)
            .field("version", &self.version)
            .finish()
    }
}

impl LegacyNode {
    pub fn parse_host(raw: &str) -> Result<NetworkAddress, MixnodeConversionError> {
        // safety: this conversion is infallible
        // (but we retain result return type for legacy reasons)
        Ok(raw.parse().unwrap())
    }

    pub fn extract_mix_host(
        host: &NetworkAddress,
        mix_port: u16,
    ) -> Result<SocketAddr, MixnodeConversionError> {
        Ok(host.to_socket_addrs(mix_port).map_err(|err| {
            MixnodeConversionError::InvalidAddress {
                value: host.to_string(),
                source: err,
            }
        })?[0])
    }
}

impl filter::Versioned for LegacyNode {
    fn version(&self) -> String {
        // TODO: return semver instead
        self.version.to_string()
    }
}

impl<'a> From<&'a LegacyNode> for SphinxNode {
    fn from(node: &'a LegacyNode) -> Self {
        let node_address_bytes = NymNodeRoutingAddress::from(node.mix_host)
            .try_into()
            .unwrap();

        SphinxNode::new(node_address_bytes, (&node.sphinx_key).into())
    }
}

impl<'a> TryFrom<&'a SkimmedNode> for LegacyNode {
    type Error = MixnodeConversionError;

    fn try_from(value: &'a SkimmedNode) -> Result<Self, Self::Error> {
        if value.ip_addresses.is_empty() {
            return Err(MixnodeConversionError::NoIpAddressesProvided {
                mixnode: value.ed25519_identity_pubkey.to_base58_string(),
            });
        }

        let layer = match value.role {
            NodeRole::Mixnode { layer } => layer
                .try_into()
                .map_err(|_| MixnodeConversionError::InvalidLayer)?,
            _ => return Err(MixnodeConversionError::NotMixnode),
        };

        // safety: we just checked the slice is not empty
        #[allow(clippy::unwrap_used)]
        let ip = value.ip_addresses.choose(&mut thread_rng()).unwrap();

        let host = NetworkAddress::IpAddr(*ip);

        Ok(LegacyNode {
            mix_id: value.node_id,
            host,
            mix_host: SocketAddr::new(*ip, value.mix_port),
            identity_key: value.ed25519_identity_pubkey,
            sphinx_key: value.x25519_sphinx_pubkey,
            layer,
            version: NodeVersion::Unknown,
        })
    }
}

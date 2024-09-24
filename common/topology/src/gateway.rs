// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{filter, NetworkAddress, NodeVersion};
use nym_api_requests::nym_nodes::SkimmedNode;
use nym_crypto::asymmetric::{encryption, identity};
use nym_mixnet_contract_common::NodeId;
use nym_sphinx_addressing::nodes::{NodeIdentity, NymNodeRoutingAddress};
use nym_sphinx_types::Node as SphinxNode;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::fmt;
use std::fmt::Formatter;
use std::io;
use std::net::AddrParseError;
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

    #[error("'{gateway}' has not provided any valid ip addresses")]
    NoIpAddressesProvided { gateway: String },

    #[error("'{gateway}' has provided a malformed ip address: {err}")]
    MalformedIpAddress {
        gateway: String,

        #[source]
        err: AddrParseError,
    },

    #[error("provided node is not an entry gateway in this epoch!")]
    NotGateway,
}

#[derive(Clone)]
pub struct LegacyNode {
    pub node_id: NodeId,

    pub host: NetworkAddress,
    // we're keeping this as separate resolved field since we do not want to be resolving the potential
    // hostname every time we want to construct a path via this node
    pub mix_host: SocketAddr,

    // #[serde(alias = "clients_port")]
    pub clients_ws_port: u16,

    // #[serde(default)]
    pub clients_wss_port: Option<u16>,

    pub identity_key: identity::PublicKey,
    pub sphinx_key: encryption::PublicKey, // TODO: or nymsphinx::PublicKey? both are x25519

    // to be removed:
    pub version: NodeVersion,
}

impl std::fmt::Debug for LegacyNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("gateway::Node")
            .field("host", &self.host)
            .field("mix_host", &self.mix_host)
            .field("clients_ws_port", &self.clients_ws_port)
            .field("clients_wss_port", &self.clients_wss_port)
            .field("identity_key", &self.identity_key.to_base58_string())
            .field("sphinx_key", &self.sphinx_key.to_base58_string())
            .field("version", &self.version)
            .finish()
    }
}

impl LegacyNode {
    pub fn parse_host(raw: &str) -> Result<NetworkAddress, GatewayConversionError> {
        // safety: this conversion is infallible
        // (but we retain result return type for legacy reasons)
        Ok(raw.parse().unwrap())
    }

    pub fn extract_mix_host(
        host: &NetworkAddress,
        mix_port: u16,
    ) -> Result<SocketAddr, GatewayConversionError> {
        Ok(host.to_socket_addrs(mix_port).map_err(|err| {
            GatewayConversionError::InvalidAddress {
                value: host.to_string(),
                source: err,
            }
        })?[0])
    }

    pub fn identity(&self) -> &NodeIdentity {
        &self.identity_key
    }

    pub fn clients_address(&self) -> String {
        self.clients_address_tls()
            .unwrap_or_else(|| self.clients_address_no_tls())
    }

    pub fn clients_address_no_tls(&self) -> String {
        format!("ws://{}:{}", self.host, self.clients_ws_port)
    }

    pub fn clients_address_tls(&self) -> Option<String> {
        self.clients_wss_port
            .map(|p| format!("wss://{}:{p}", self.host))
    }
}

impl fmt::Display for LegacyNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "legacy gateway {} @ {}", self.node_id, self.host)
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
    type Error = GatewayConversionError;

    fn try_from(value: &'a SkimmedNode) -> Result<Self, Self::Error> {
        let Some(entry_details) = &value.entry else {
            return Err(GatewayConversionError::NotGateway);
        };

        if value.ip_addresses.is_empty() {
            return Err(GatewayConversionError::NoIpAddressesProvided {
                gateway: value.ed25519_identity_pubkey.to_base58_string(),
            });
        }

        // safety: we just checked the slice is not empty
        #[allow(clippy::unwrap_used)]
        let ip = value.ip_addresses.choose(&mut thread_rng()).unwrap();

        let host = if let Some(hostname) = &entry_details.hostname {
            NetworkAddress::Hostname(hostname.to_string())
        } else {
            NetworkAddress::IpAddr(*ip)
        };

        Ok(LegacyNode {
            node_id: value.node_id,
            host,
            mix_host: SocketAddr::new(*ip, value.mix_port),
            clients_ws_port: entry_details.ws_port,
            clients_wss_port: entry_details.wss_port,
            identity_key: value.ed25519_identity_pubkey,
            sphinx_key: value.x25519_sphinx_pubkey,
            version: NodeVersion::Unknown,
        })
    }
}

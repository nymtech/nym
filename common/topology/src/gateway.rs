// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{filter, NetworkAddress, NodeVersion};
use nym_api_requests::models::DescribedGateway;
use nym_crypto::asymmetric::{encryption, identity};
use nym_mixnet_contract_common::GatewayBond;
use nym_sphinx_addressing::nodes::{NodeIdentity, NymNodeRoutingAddress};
use nym_sphinx_types::Node as SphinxNode;

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
}

#[derive(Clone)]
pub struct Node {
    pub owner: String,
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
    pub version: NodeVersion,
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("gateway::Node")
            .field("host", &self.host)
            .field("owner", &self.owner)
            .field("mix_host", &self.mix_host)
            .field("clients_ws_port", &self.clients_ws_port)
            .field("clients_wss_port", &self.clients_wss_port)
            .field("identity_key", &self.identity_key.to_base58_string())
            .field("sphinx_key", &self.sphinx_key.to_base58_string())
            .field("version", &self.version)
            .finish()
    }
}

impl Node {
    pub fn parse_host(raw: &str) -> Result<NetworkAddress, GatewayConversionError> {
        raw.parse()
            .map_err(|err| GatewayConversionError::InvalidAddress {
                value: raw.to_owned(),
                source: err,
            })
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

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Node(id: {}, owner: {}, host: {})",
            self.identity_key, self.owner, self.host,
        )
    }
}

impl filter::Versioned for Node {
    fn version(&self) -> String {
        // TODO: return semver instead
        self.version.to_string()
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
        let host = Self::parse_host(&bond.gateway.host)?;

        // try to completely resolve the host in the mix situation to avoid doing it every
        // single time we want to construct a path
        let mix_host = Self::extract_mix_host(&host, bond.gateway.mix_port)?;

        Ok(Node {
            owner: bond.owner.as_str().to_owned(),
            host,
            mix_host,
            clients_ws_port: bond.gateway.clients_port,
            clients_wss_port: None,
            identity_key: identity::PublicKey::from_base58_string(&bond.gateway.identity_key)?,
            sphinx_key: encryption::PublicKey::from_base58_string(&bond.gateway.sphinx_key)?,
            version: bond.gateway.version.as_str().into(),
        })
    }
}

impl TryFrom<GatewayBond> for Node {
    type Error = GatewayConversionError;

    fn try_from(bond: GatewayBond) -> Result<Self, Self::Error> {
        Node::try_from(&bond)
    }
}

impl<'a> TryFrom<&'a DescribedGateway> for Node {
    type Error = GatewayConversionError;

    fn try_from(value: &'a DescribedGateway) -> Result<Self, Self::Error> {
        let Some(ref self_described) = value.self_described else {
            return (&value.bond).try_into();
        };

        let ips = &self_described.host_information.ip_address;
        if ips.is_empty() {
            return Err(GatewayConversionError::NoIpAddressesProvided {
                gateway: value.bond.gateway.identity_key.clone(),
            });
        }

        let host = match &self_described.host_information.hostname {
            None => NetworkAddress::IpAddr(ips[0]),
            Some(hostname) => NetworkAddress::Hostname(hostname.clone()),
        };

        // get ip from the self-reported values so we wouldn't need to do any hostname resolution
        // (which doesn't really work in wasm)
        let mix_host = SocketAddr::new(ips[0], value.bond.gateway.mix_port);

        Ok(Node {
            owner: value.bond.owner.as_str().to_owned(),
            host,
            mix_host,
            clients_ws_port: self_described.mixnet_websockets.ws_port,
            clients_wss_port: self_described.mixnet_websockets.wss_port,
            identity_key: identity::PublicKey::from_base58_string(
                &self_described.host_information.keys.ed25519,
            )?,
            sphinx_key: encryption::PublicKey::from_base58_string(
                &self_described.host_information.keys.x25519,
            )?,
            version: self_described
                .build_information
                .build_version
                .as_str()
                .into(),
        })
    }
}

impl TryFrom<DescribedGateway> for Node {
    type Error = GatewayConversionError;

    fn try_from(value: DescribedGateway) -> Result<Self, Self::Error> {
        Node::try_from(&value)
    }
}

// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_api_requests::models::DeclaredRoles;
use nym_api_requests::nym_nodes::SkimmedNode;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_mixnet_contract_common::NodeId;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_types::Node as SphinxNode;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use thiserror::Error;

pub use nym_mixnet_contract_common::LegacyMixLayer;

#[derive(Error, Debug)]
pub enum RoutingNodeError {
    #[error("this node has no mixing information available")]
    NoMixingInformationAvailable,

    #[error("node {node_id} ('{identity}') has not provided any valid ip addresses")]
    NoIpAddressesProvided {
        node_id: NodeId,
        identity: ed25519::PublicKey,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntryDetails {
    // to allow client to choose ipv6 preference, if available
    pub ip_addresses: Vec<IpAddr>,
    pub clients_ws_port: u16,
    pub hostname: Option<String>,
    pub clients_wss_port: Option<u16>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SupportedRoles {
    pub mixnode: bool,
    pub mixnet_entry: bool,
    pub mixnet_exit: bool,
}

impl From<DeclaredRoles> for SupportedRoles {
    fn from(value: DeclaredRoles) -> Self {
        SupportedRoles {
            mixnode: value.mixnode,
            mixnet_entry: value.entry,
            mixnet_exit: value.exit_nr && value.exit_ipr,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoutingNode {
    pub node_id: NodeId,

    pub mix_host: SocketAddr,

    pub entry: Option<EntryDetails>,
    pub identity_key: ed25519::PublicKey,
    pub sphinx_key: x25519::PublicKey,

    pub supported_roles: SupportedRoles,
}

impl RoutingNode {
    pub fn ws_entry_address_tls(&self) -> Option<String> {
        let entry = self.entry.as_ref()?;
        let hostname = entry.hostname.as_ref()?;
        let wss_port = entry.clients_wss_port?;

        Some(format!("wss://{hostname}:{wss_port}"))
    }

    pub fn ws_entry_address_no_tls(&self, prefer_ipv6: bool) -> Option<String> {
        let entry = self.entry.as_ref()?;

        if let Some(hostname) = entry.hostname.as_ref() {
            return Some(format!("ws://{hostname}:{}", entry.clients_ws_port));
        }

        if prefer_ipv6 {
            if let Some(ipv6) = entry.ip_addresses.iter().find(|ip| ip.is_ipv6()) {
                return Some(format!("ws://{ipv6}:{}", entry.clients_ws_port));
            }
        }

        let any_ip = entry.ip_addresses.first()?;
        Some(format!("ws://{any_ip}:{}", entry.clients_ws_port))
    }

    pub fn ws_entry_address(&self, prefer_ipv6: bool) -> Option<String> {
        if let Some(tls) = self.ws_entry_address_tls() {
            return Some(tls);
        }
        self.ws_entry_address_no_tls(prefer_ipv6)
    }

    pub fn identity(&self) -> ed25519::PublicKey {
        self.identity_key
    }
}

impl<'a> From<&'a RoutingNode> for SphinxNode {
    fn from(node: &'a RoutingNode) -> Self {
        // SAFETY: this conversion is infallible as all versions of socket addresses have
        // sufficiently small bytes representation to fit inside `NodeAddressBytes`
        #[allow(clippy::unwrap_used)]
        let node_address_bytes = NymNodeRoutingAddress::from(node.mix_host)
            .try_into()
            .unwrap();

        SphinxNode::new(node_address_bytes, (&node.sphinx_key).into())
    }
}

impl<'a> TryFrom<&'a SkimmedNode> for RoutingNode {
    type Error = RoutingNodeError;

    fn try_from(value: &'a SkimmedNode) -> Result<Self, Self::Error> {
        // IF YOU EVER ADD "performance" TO RoutingNode,
        // MAKE SURE TO UPDATE THE LAZY IMPLEMENTATION OF
        // `impl NodeDescriptionTopologyExt for NymNodeDescription`!!!

        let Some(first_ip) = value.ip_addresses.first() else {
            return Err(RoutingNodeError::NoIpAddressesProvided {
                node_id: value.node_id,
                identity: value.ed25519_identity_pubkey,
            });
        };

        let entry = match &value.entry {
            None => None,
            Some(entry) => Some(EntryDetails {
                ip_addresses: value.ip_addresses.clone(),
                clients_ws_port: entry.ws_port,
                hostname: entry.hostname.clone(),
                clients_wss_port: entry.wss_port,
            }),
        };

        Ok(RoutingNode {
            node_id: value.node_id,
            mix_host: SocketAddr::new(*first_ip, value.mix_port),
            entry,
            identity_key: value.ed25519_identity_pubkey,
            sphinx_key: value.x25519_sphinx_pubkey,
            supported_roles: value.supported_roles.into(),
        })
    }
}

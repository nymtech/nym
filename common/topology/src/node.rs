// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_api_requests::models::DeclaredRoles;
use nym_api_requests::nym_nodes::SkimmedNode;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_mixnet_contract_common::NodeId;
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_types::Node as SphinxNode;
use std::net::SocketAddr;
use thiserror::Error;

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

#[derive(Debug, Clone)]
pub struct EntryDetails {
    pub clients_ws_port: u16,
    pub hostname: Option<String>,
    pub clients_wss_port: Option<u16>,
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone)]
pub struct RoutingNode {
    pub node_id: NodeId,

    pub mix_host: SocketAddr,

    pub entry: Option<EntryDetails>,
    pub identity_key: ed25519::PublicKey,
    pub sphinx_key: x25519::PublicKey,

    pub supported_roles: SupportedRoles,
    pub performance: f64,
}

impl RoutingNode {
    pub fn ws_entry_address_tls(&self) -> Option<String> {
        todo!()
    }

    pub fn ws_entry_address_no_tls(&self) -> Option<String> {
        todo!()
    }

    pub fn ws_entry_address(&self) -> Option<String> {
        if let Some(tls) = self.ws_entry_address_tls() {
            return Some(tls);
        }
        self.ws_entry_address_no_tls()
    }

    pub fn identity(&self) -> ed25519::PublicKey {
        self.identity_key
    }

    pub fn mix_host(&self) -> Option<SocketAddr> {
        todo!()
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
        if value.ip_addresses.is_empty() {
            return Err(RoutingNodeError::NoIpAddressesProvided {
                node_id: value.node_id,
                identity: value.ed25519_identity_pubkey,
            });
        }
        todo!()
    }
}

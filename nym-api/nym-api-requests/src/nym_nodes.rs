// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::{DeclaredRoles, NymNodeData, OffsetDateTimeJsonSchemaWrapper};
use crate::pagination::{PaginatedResponse, Pagination};
use nym_crypto::asymmetric::ed25519::serde_helpers::bs58_ed25519_pubkey;
use nym_crypto::asymmetric::x25519::serde_helpers::bs58_x25519_pubkey;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::NodeId;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use time::OffsetDateTime;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CachedNodesResponse<T> {
    pub refreshed_at: OffsetDateTimeJsonSchemaWrapper,
    pub nodes: Vec<T>,
}

impl<T> From<Vec<T>> for CachedNodesResponse<T> {
    fn from(nodes: Vec<T>) -> Self {
        CachedNodesResponse::new(nodes)
    }
}

impl<T> CachedNodesResponse<T> {
    pub fn new(nodes: Vec<T>) -> Self {
        CachedNodesResponse {
            refreshed_at: OffsetDateTime::now_utc().into(),
            nodes,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PaginatedCachedNodesResponse<T> {
    pub refreshed_at: OffsetDateTimeJsonSchemaWrapper,
    pub nodes: PaginatedResponse<T>,
}

impl<T> PaginatedCachedNodesResponse<T> {
    pub fn new_full(
        refreshed_at: impl Into<OffsetDateTimeJsonSchemaWrapper>,
        nodes: Vec<T>,
    ) -> Self {
        PaginatedCachedNodesResponse {
            refreshed_at: refreshed_at.into(),
            nodes: PaginatedResponse {
                pagination: Pagination {
                    total: nodes.len(),
                    page: 0,
                    size: nodes.len(),
                },
                data: nodes,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, utoipa::ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum NodeRoleQueryParam {
    ActiveMixnode,

    #[serde(alias = "entry", alias = "gateway")]
    EntryGateway,

    #[serde(alias = "exit")]
    ExitGateway,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub enum NodeRole {
    // a properly active mixnode
    Mixnode {
        layer: u8,
    },

    #[serde(alias = "entry", alias = "gateway")]
    EntryGateway,

    #[serde(alias = "exit")]
    ExitGateway,

    // equivalent of node that's in rewarded set but not in the inactive set
    Standby,

    Inactive,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct BasicEntryInformation {
    pub hostname: Option<String>,

    pub ws_port: u16,
    pub wss_port: Option<u16>,
}

// the bare minimum information needed to construct sphinx packets
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct SkimmedNode {
    // in directory v3 all nodes (mixnodes AND gateways) will have a unique id
    // but to keep structure consistent, introduce this field now
    #[schema(value_type = u32)]
    pub node_id: NodeId,

    #[serde(with = "bs58_ed25519_pubkey")]
    #[schemars(with = "String")]
    pub ed25519_identity_pubkey: ed25519::PublicKey,

    #[schema(value_type = Vec<String>)]
    pub ip_addresses: Vec<IpAddr>,

    // TODO: to be deprecated in favour of well-known hardcoded port for everyone
    pub mix_port: u16,

    #[serde(with = "bs58_x25519_pubkey")]
    #[schemars(with = "String")]
    pub x25519_sphinx_pubkey: x25519::PublicKey,

    #[serde(alias = "role")]
    pub epoch_role: NodeRole,

    // needed for the purposes of sending appropriate test packets
    #[serde(default)]
    pub supported_roles: DeclaredRoles,

    pub entry: Option<BasicEntryInformation>,

    /// Average node performance in last 24h period
    #[schema(value_type = String)]
    pub performance: Performance,
}

impl SkimmedNode {
    pub fn get_mix_layer(&self) -> Option<u8> {
        match self.epoch_role {
            NodeRole::Mixnode { layer } => Some(layer),
            _ => None,
        }
    }
}

// an intermediate variant that exposes additional data such as noise keys but without
// the full fat of the self-described data
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct SemiSkimmedNode {
    pub basic: SkimmedNode,
    pub x25519_noise_pubkey: String,
    // pub location:
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct FullFatNode {
    pub expanded: SemiSkimmedNode,

    // kinda temporary for now to make as few changes as possible for now
    pub self_described: Option<NymNodeData>,
}

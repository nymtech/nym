// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::{DeclaredRoles, NymNodeData, OffsetDateTimeJsonSchemaWrapper};
use crate::pagination::{PaginatedResponse, Pagination};
use nym_crypto::asymmetric::ed25519::serde_helpers::bs58_ed25519_pubkey;
use nym_crypto::asymmetric::x25519::serde_helpers::bs58_x25519_pubkey;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_mixnet_contract_common::nym_node::Role;
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::{EpochId, Interval, NodeId};
use nym_noise_keys::VersionedNoiseKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use time::OffsetDateTime;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, utoipa::ToSchema)]
pub struct SkimmedNodesWithMetadata {
    pub nodes: Vec<SkimmedNode>,
    pub metadata: NodesResponseMetadata,
}

impl SkimmedNodesWithMetadata {
    pub fn new(nodes: Vec<SkimmedNode>, metadata: NodesResponseMetadata) -> Self {
        SkimmedNodesWithMetadata { nodes, metadata }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, utoipa::ToSchema)]
pub struct SemiSkimmedNodesWithMetadata {
    pub nodes: Vec<SemiSkimmedNode>,
    pub metadata: NodesResponseMetadata,
}

impl SemiSkimmedNodesWithMetadata {
    pub fn new(nodes: Vec<SemiSkimmedNode>, metadata: NodesResponseMetadata) -> Self {
        SemiSkimmedNodesWithMetadata { nodes, metadata }
    }
}

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, utoipa::ToSchema, PartialEq,
)]
#[serde(rename_all = "kebab-case")]
pub enum TopologyRequestStatus {
    NoUpdates,
    Fresh(Interval),
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct CachedNodesResponse<T: ToSchema> {
    pub refreshed_at: OffsetDateTimeJsonSchemaWrapper,
    pub nodes: Vec<T>,
}

impl<T: ToSchema> From<Vec<T>> for CachedNodesResponse<T> {
    fn from(nodes: Vec<T>) -> Self {
        CachedNodesResponse::new(nodes)
    }
}

impl<T: ToSchema> CachedNodesResponse<T> {
    pub fn new(nodes: Vec<T>) -> Self {
        CachedNodesResponse {
            refreshed_at: OffsetDateTime::now_utc().into(),
            nodes,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, utoipa::ToSchema)]
pub struct NodesResponseMetadata {
    pub status: Option<TopologyRequestStatus>,
    #[schema(value_type = u32)]
    pub absolute_epoch_id: EpochId,
    pub rotation_id: u32,
    pub refreshed_at: OffsetDateTimeJsonSchemaWrapper,
}

impl NodesResponseMetadata {
    pub fn consistency_check(&self, other: &NodesResponseMetadata) -> bool {
        self.status == other.status
            && self.absolute_epoch_id == other.absolute_epoch_id
            && self.rotation_id == other.rotation_id
    }

    pub fn refreshed_at(&self) -> OffsetDateTime {
        self.refreshed_at.into()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
// can't add any new fields here, even with #[serde(default)] and whatnot,
// because it will break all clients using bincode : (
pub struct PaginatedCachedNodesResponseV1<T> {
    pub status: Option<TopologyRequestStatus>,
    pub refreshed_at: OffsetDateTimeJsonSchemaWrapper,
    pub nodes: PaginatedResponse<T>,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PaginatedCachedNodesResponseV2<T> {
    pub metadata: NodesResponseMetadata,
    pub nodes: PaginatedResponse<T>,
}

impl<T> From<PaginatedCachedNodesResponseV2<T>> for PaginatedCachedNodesResponseV1<T> {
    fn from(res: PaginatedCachedNodesResponseV2<T>) -> Self {
        PaginatedCachedNodesResponseV1 {
            status: res.metadata.status,
            refreshed_at: res.metadata.refreshed_at,
            nodes: res.nodes,
        }
    }
}

impl<T> PaginatedCachedNodesResponseV2<T> {
    pub fn new_full(
        absolute_epoch_id: EpochId,
        rotation_id: u32,
        refreshed_at: impl Into<OffsetDateTimeJsonSchemaWrapper>,
        nodes: Vec<T>,
    ) -> Self {
        PaginatedCachedNodesResponseV2 {
            nodes: PaginatedResponse {
                pagination: Pagination {
                    total: nodes.len(),
                    page: 0,
                    size: nodes.len(),
                },
                data: nodes,
            },
            metadata: NodesResponseMetadata {
                refreshed_at: refreshed_at.into(),
                status: None,
                absolute_epoch_id,
                rotation_id,
            },
        }
    }

    pub fn fresh(mut self, interval: Interval) -> Self {
        self.metadata.status = Some(TopologyRequestStatus::Fresh(interval));
        self
    }

    pub fn no_updates(absolute_epoch_id: EpochId, rotation_id: u32) -> Self {
        PaginatedCachedNodesResponseV2 {
            nodes: PaginatedResponse {
                pagination: Pagination {
                    total: 0,
                    page: 0,
                    size: 0,
                },
                data: Vec::new(),
            },
            metadata: NodesResponseMetadata {
                refreshed_at: OffsetDateTime::now_utc().into(),
                status: Some(TopologyRequestStatus::NoUpdates),
                absolute_epoch_id,
                rotation_id,
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

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema, Default)]
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

    #[default]
    Inactive,
}

impl NodeRole {
    pub fn is_inactive(&self) -> bool {
        matches!(self, NodeRole::Inactive)
    }
}

impl From<Option<Role>> for NodeRole {
    fn from(role: Option<Role>) -> Self {
        match role {
            Some(Role::EntryGateway) => NodeRole::EntryGateway,
            Some(Role::Layer1) => NodeRole::Mixnode { layer: 1 },
            Some(Role::Layer2) => NodeRole::Mixnode { layer: 2 },
            Some(Role::Layer3) => NodeRole::Mixnode { layer: 3 },
            Some(Role::ExitGateway) => NodeRole::ExitGateway,
            Some(Role::Standby) => NodeRole::Standby,
            None => NodeRole::Inactive,
        }
    }
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
    #[schema(value_type = String)]
    pub ed25519_identity_pubkey: ed25519::PublicKey,

    #[schema(value_type = Vec<String>)]
    pub ip_addresses: Vec<IpAddr>,

    pub mix_port: u16,

    #[serde(with = "bs58_x25519_pubkey")]
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub x25519_sphinx_pubkey: x25519::PublicKey,

    #[serde(alias = "epoch_role")]
    pub role: NodeRole,

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
        match self.role {
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

    pub x25519_noise_versioned_key: Option<VersionedNoiseKey>,
    // pub location:
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct FullFatNode {
    pub expanded: SemiSkimmedNode,

    // kinda temporary for now to make as few changes as possible for now
    pub self_described: Option<NymNodeData>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, ToSchema)]
pub struct NodesByAddressesRequestBody {
    #[schema(value_type = Vec<String>)]
    pub addresses: Vec<IpAddr>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, ToSchema)]
pub struct NodesByAddressesResponse {
    #[schema(value_type = HashMap<String, Option<u32>>)]
    pub existence: HashMap<IpAddr, Option<NodeId>>,
}

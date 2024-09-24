// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::{
    DeclaredRoles, GatewayBondAnnotated, MixNodeBondAnnotated, NymNodeData,
    OffsetDateTimeJsonSchemaWrapper,
};
use crate::pagination::PaginatedResponse;
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

    // pub fn from_described_gateway(
    //     annotated: &GatewayBondAnnotated,
    //     description: Option<&NymNodeData>,
    // ) -> Self {
    //     let mut base: SkimmedNode = annotated.into();
    //     let Some(description) = description else {
    //         return base;
    //     };
    //
    //     // safety: the conversion always sets the entry field
    //     let entry = base.entry.as_mut().unwrap();
    //     entry
    //         .hostname
    //         .clone_from(&description.host_information.hostname);
    //     entry.ws_port = description.mixnet_websockets.ws_port;
    //     entry.wss_port = description.mixnet_websockets.wss_port;
    //
    //     // always prefer self-described data
    //     if !description.host_information.ip_address.is_empty() {
    //         base.ip_addresses
    //             .clone_from(&description.host_information.ip_address)
    //     }
    //
    //     base.supported_roles = description.declared_role;
    //
    //     base
    // }
}

// impl<'a> From<&'a MixNodeBondAnnotated> for SkimmedNode {
//     fn from(value: &'a MixNodeBondAnnotated) -> Self {
//         todo!()
//         // SkimmedNode {
//         //     node_id: value.mix_id(),
//         //     ed25519_identity_pubkey: value.identity_key().to_string(),
//         //     ip_addresses: value.ip_addresses.clone(),
//         //     mix_port: value.mix_node().mix_port,
//         //     x25519_sphinx_pubkey: value.mix_node().sphinx_key.clone(),
//         //     epoch_role: NodeRole::Mixnode {
//         //         layer: value.mixnode_details.bond_information.layer.into(),
//         //     },
//         //     supported_roles: DeclaredRoles {
//         //         mixnode: true,
//         //         entry: false,
//         //         exit_nr: false,
//         //         exit_ipr: false,
//         //     },
//         //     entry: None,
//         //     performance: value.node_performance.last_24h,
//         // }
//     }
// }

// impl<'a> From<&'a GatewayBondAnnotated> for SkimmedNode {
//     fn from(value: &'a GatewayBondAnnotated) -> Self {
//         todo!()
//         // SkimmedNode {
//         //     node_id: value.gateway_bond.node_id,
//         //     ip_addresses: value.ip_addresses.clone(),
//         //     ed25519_identity_pubkey: value.gateway_bond.bond.identity().clone(),
//         //     mix_port: value.gateway_bond.bond.gateway.mix_port,
//         //     x25519_sphinx_pubkey: value.gateway_bond.bond.gateway.sphinx_key.clone(),
//         //     epoch_role: NodeRole::EntryGateway,
//         //     supported_roles: DeclaredRoles {
//         //         mixnode: false,
//         //         entry: true,
//         //         exit_nr: false,
//         //         exit_ipr: false,
//         //     },
//         //     entry: Some(BasicEntryInformation {
//         //         hostname: None,
//         //         ws_port: value.gateway_bond.bond.gateway.clients_port,
//         //         wss_port: None,
//         //     }),
//         //     performance: value.node_performance.last_24h,
//         // }
//     }
// }

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

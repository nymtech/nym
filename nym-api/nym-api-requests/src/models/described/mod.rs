// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::{BinaryBuildInformationOwned, OffsetDateTimeJsonSchemaWrapper};
use crate::nym_nodes::{BasicEntryInformation, NodeRole, SemiSkimmedNode, SkimmedNode};

use nym_crypto::asymmetric::{ed25519, x25519};
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::NodeId;
use nym_network_defaults::{DEFAULT_MIX_LISTENING_PORT, DEFAULT_VERLOC_LISTENING_PORT};
use nym_noise_keys::VersionedNoiseKey;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use tracing::warn;
use utoipa::ToSchema;

pub mod type_translation;

// don't break existing imports
pub use type_translation::*;

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct NoiseDetails {
    pub key: VersionedNoiseKey,

    pub mixnet_port: u16,

    #[schema(value_type = Vec<String>)]
    pub ip_addresses: Vec<IpAddr>,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct NymNodeDescription {
    #[schema(value_type = u32)]
    pub node_id: NodeId,
    pub contract_node_type: DescribedNodeType,
    pub description: NymNodeData,
}

impl NymNodeDescription {
    pub fn version(&self) -> &str {
        &self.description.build_information.build_version
    }

    pub fn entry_information(&self) -> BasicEntryInformation {
        BasicEntryInformation {
            hostname: self.description.host_information.hostname.clone(),
            ws_port: self.description.mixnet_websockets.ws_port,
            wss_port: self.description.mixnet_websockets.wss_port,
        }
    }

    pub fn ed25519_identity_key(&self) -> ed25519::PublicKey {
        self.description.host_information.keys.ed25519
    }

    pub fn current_sphinx_key(&self, current_rotation_id: u32) -> x25519::PublicKey {
        let keys = &self.description.host_information.keys;

        if keys.current_x25519_sphinx_key.rotation_id == u32::MAX {
            // legacy case (i.e. node doesn't support rotation)
            return keys.current_x25519_sphinx_key.public_key;
        }

        if current_rotation_id == keys.current_x25519_sphinx_key.rotation_id {
            // it's the 'current' key
            return keys.current_x25519_sphinx_key.public_key;
        }

        if let Some(pre_announced) = &keys.pre_announced_x25519_sphinx_key {
            if pre_announced.rotation_id == current_rotation_id {
                return pre_announced.public_key;
            }
        }

        warn!(
            "unexpected key rotation {current_rotation_id} for node {}",
            self.node_id
        );
        // this should never be reached, but just in case, return the fallback option
        keys.current_x25519_sphinx_key.public_key
    }

    pub fn to_skimmed_node(
        &self,
        current_rotation_id: u32,
        role: NodeRole,
        performance: Performance,
    ) -> SkimmedNode {
        let keys = &self.description.host_information.keys;
        let entry = if self.description.declared_role.entry {
            Some(self.entry_information())
        } else {
            None
        };

        SkimmedNode {
            node_id: self.node_id,
            ed25519_identity_pubkey: keys.ed25519,
            ip_addresses: self.description.host_information.ip_address.clone(),
            mix_port: self.description.mix_port(),
            x25519_sphinx_pubkey: self.current_sphinx_key(current_rotation_id),
            // we can't use the declared roles, we have to take whatever was provided in the contract.
            // why? say this node COULD operate as an exit, but it might be the case the contract decided
            // to assign it an ENTRY role only. we have to use that one instead.
            role,
            supported_roles: self.description.declared_role,
            entry,
            performance,
        }
    }

    pub fn to_semi_skimmed_node(
        &self,
        current_rotation_id: u32,
        role: NodeRole,
        performance: Performance,
    ) -> SemiSkimmedNode {
        let skimmed_node = self.to_skimmed_node(current_rotation_id, role, performance);

        SemiSkimmedNode {
            basic: skimmed_node,
            x25519_noise_versioned_key: self
                .description
                .host_information
                .keys
                .x25519_versioned_noise,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/DescribedNodeType.ts"
    )
)]
pub enum DescribedNodeType {
    LegacyMixnode,
    LegacyGateway,
    NymNode,
}

impl DescribedNodeType {
    pub fn is_nym_node(&self) -> bool {
        matches!(self, DescribedNodeType::NymNode)
    }
}

// this struct is getting quite bloated...
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct NymNodeData {
    #[serde(default)]
    pub last_polled: OffsetDateTimeJsonSchemaWrapper,

    pub host_information: HostInformation,

    #[serde(default)]
    pub declared_role: DeclaredRoles,

    #[serde(default)]
    pub auxiliary_details: AuxiliaryDetails,

    // TODO: do we really care about ALL build info or just the version?
    pub build_information: BinaryBuildInformationOwned,

    #[serde(default)]
    pub network_requester: Option<NetworkRequesterDetails>,

    #[serde(default)]
    pub ip_packet_router: Option<IpPacketRouterDetails>,

    #[serde(default)]
    pub authenticator: Option<AuthenticatorDetails>,

    #[serde(default)]
    pub wireguard: Option<WireguardDetails>,

    #[serde(default)]
    pub lewes_protocol: Option<LewesProtocolDetails>,

    // for now we only care about their ws/wss situation, nothing more
    pub mixnet_websockets: WebSockets,
}

impl NymNodeData {
    pub fn mix_port(&self) -> u16 {
        self.auxiliary_details
            .announce_ports
            .mix_port
            .unwrap_or(DEFAULT_MIX_LISTENING_PORT)
    }

    pub fn verloc_port(&self) -> u16 {
        self.auxiliary_details
            .announce_ports
            .verloc_port
            .unwrap_or(DEFAULT_VERLOC_LISTENING_PORT)
    }
}

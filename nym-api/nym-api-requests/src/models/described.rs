// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::{BinaryBuildInformationOwned, OffsetDateTimeJsonSchemaWrapper};
use crate::nym_nodes::{BasicEntryInformation, NodeRole, SemiSkimmedNode, SkimmedNode};
use nym_crypto::asymmetric::ed25519::serde_helpers::bs58_ed25519_pubkey;
use nym_crypto::asymmetric::x25519::serde_helpers::bs58_x25519_pubkey;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::NodeId;
use nym_network_defaults::{DEFAULT_MIX_LISTENING_PORT, DEFAULT_VERLOC_LISTENING_PORT};
use nym_node_requests::api::v1::authenticator::models::Authenticator;
use nym_node_requests::api::v1::gateway::models::Wireguard;
use nym_node_requests::api::v1::ip_packet_router::models::IpPacketRouter;
use nym_node_requests::api::v1::node::models::{AuxiliaryDetails, NodeRoles};
use nym_noise_keys::VersionedNoiseKey;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use tracing::warn;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct HostInformation {
    #[schema(value_type = Vec<String>)]
    pub ip_address: Vec<IpAddr>,
    pub hostname: Option<String>,
    pub keys: HostKeys,
}

impl From<nym_node_requests::api::v1::node::models::HostInformation> for HostInformation {
    fn from(value: nym_node_requests::api::v1::node::models::HostInformation) -> Self {
        HostInformation {
            ip_address: value.ip_address,
            hostname: value.hostname,
            keys: value.keys.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct HostKeys {
    #[serde(with = "bs58_ed25519_pubkey")]
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub ed25519: ed25519::PublicKey,

    #[deprecated(note = "use the current_x25519_sphinx_key with explicit rotation information")]
    #[serde(with = "bs58_x25519_pubkey")]
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub x25519: x25519::PublicKey,

    pub current_x25519_sphinx_key: SphinxKey,

    #[serde(default)]
    pub pre_announced_x25519_sphinx_key: Option<SphinxKey>,

    #[serde(default)]
    pub x25519_versioned_noise: Option<VersionedNoiseKey>,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct SphinxKey {
    pub rotation_id: u32,

    #[serde(with = "bs58_x25519_pubkey")]
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub public_key: x25519::PublicKey,
}

impl From<nym_node_requests::api::v1::node::models::SphinxKey> for SphinxKey {
    fn from(value: nym_node_requests::api::v1::node::models::SphinxKey) -> Self {
        SphinxKey {
            rotation_id: value.rotation_id,
            public_key: value.public_key,
        }
    }
}

impl From<nym_node_requests::api::v1::node::models::HostKeys> for HostKeys {
    fn from(value: nym_node_requests::api::v1::node::models::HostKeys) -> Self {
        HostKeys {
            ed25519: value.ed25519_identity,
            x25519: value.x25519_sphinx,
            current_x25519_sphinx_key: value.primary_x25519_sphinx_key.into(),
            pre_announced_x25519_sphinx_key: value.pre_announced_x25519_sphinx_key.map(Into::into),
            x25519_versioned_noise: value.x25519_versioned_noise,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct WebSockets {
    pub ws_port: u16,

    pub wss_port: Option<u16>,
}

impl From<nym_node_requests::api::v1::gateway::models::WebSockets> for WebSockets {
    fn from(value: nym_node_requests::api::v1::gateway::models::WebSockets) -> Self {
        WebSockets {
            ws_port: value.ws_port,
            wss_port: value.wss_port,
        }
    }
}

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

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/DeclaredRoles.ts"
    )
)]
pub struct DeclaredRoles {
    pub mixnode: bool,
    pub entry: bool,
    pub exit_nr: bool,
    pub exit_ipr: bool,
}

impl DeclaredRoles {
    pub fn can_operate_exit_gateway(&self) -> bool {
        self.exit_ipr && self.exit_nr
    }
}

impl From<NodeRoles> for DeclaredRoles {
    fn from(value: NodeRoles) -> Self {
        DeclaredRoles {
            mixnode: value.mixnode_enabled,
            entry: value.gateway_enabled,
            exit_nr: value.gateway_enabled && value.network_requester_enabled,
            exit_ipr: value.gateway_enabled && value.ip_packet_router_enabled,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct NetworkRequesterDetails {
    /// address of the embedded network requester
    pub address: String,

    /// flag indicating whether this network requester uses the exit policy rather than the deprecated allow list
    pub uses_exit_policy: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct IpPacketRouterDetails {
    /// address of the embedded ip packet router
    pub address: String,
}

// works for current simple case.
impl From<IpPacketRouter> for IpPacketRouterDetails {
    fn from(value: IpPacketRouter) -> Self {
        IpPacketRouterDetails {
            address: value.address,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct AuthenticatorDetails {
    /// address of the embedded authenticator
    pub address: String,
}

// works for current simple case.
impl From<Authenticator> for AuthenticatorDetails {
    fn from(value: Authenticator) -> Self {
        AuthenticatorDetails {
            address: value.address,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct WireguardDetails {
    pub port: u16,
    pub public_key: String,
}

// works for current simple case.
impl From<Wireguard> for WireguardDetails {
    fn from(value: Wireguard) -> Self {
        WireguardDetails {
            port: value.port,
            public_key: value.public_key,
        }
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

// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::models::described::type_translation::LewesProtocolDetailsV1;
use crate::models::described::v1::NymNodeDescriptionV1;
use crate::models::described::v2::{
    AnnouncePortsV2, AuthenticatorDetailsV2, DeclaredRolesV2, DescribedNodeTypeV2,
    HostInformationV2, HostKeysV2, IpPacketRouterDetailsV2, NetworkRequesterDetailsV2,
    NymNodeAuxiliaryDetailsV2, NymNodeDataV2, NymNodeDescriptionV2, SphinxKeyV2,
    VersionedNoiseKeyV2, WebSocketsV2, WireguardDetailsV2,
};
use crate::models::{BinaryBuildInformationOwned, OffsetDateTimeJsonSchemaWrapper};
use crate::nym_nodes::{
    BasicEntryInformation, NodeRole, SemiSkimmedNodeV1, SemiSkimmedNodeV3, SkimmedNodeV1,
};
use celes::Country;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::NodeId;
use nym_network_defaults::{DEFAULT_MIX_LISTENING_PORT, DEFAULT_VERLOC_LISTENING_PORT};
use serde::{Deserialize, Serialize};
use tracing::warn;
use utoipa::ToSchema;

// no changes for the following types
pub type AnnouncePortsV3 = AnnouncePortsV2;
pub type HostInformationV3 = HostInformationV2;
pub type DeclaredRolesV3 = DeclaredRolesV2;
pub type NetworkRequesterDetailsV3 = NetworkRequesterDetailsV2;
pub type IpPacketRouterDetailsV3 = IpPacketRouterDetailsV2;
pub type AuthenticatorDetailsV3 = AuthenticatorDetailsV2;
pub type WireguardDetailsV3 = WireguardDetailsV2;
pub type WebSocketsV3 = WebSocketsV2;
pub type DescribedNodeTypeV3 = DescribedNodeTypeV2;
pub type HostKeysV3 = HostKeysV2;
pub type SphinxKeyV3 = SphinxKeyV2;
pub type VersionedNoiseKeyV3 = VersionedNoiseKeyV2;
pub type LewesProtocolDetailsV3 = LewesProtocolDetailsV1;

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct NymNodeDescriptionV3 {
    #[schema(value_type = u32)]
    pub node_id: NodeId,
    pub contract_node_type: DescribedNodeTypeV3,
    pub description: NymNodeDataV3,
}

impl NymNodeDescriptionV3 {
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
    ) -> SkimmedNodeV1 {
        let keys = &self.description.host_information.keys;
        let entry = if self.description.declared_role.entry {
            Some(self.entry_information())
        } else {
            None
        };

        SkimmedNodeV1 {
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
    ) -> SemiSkimmedNodeV1 {
        let skimmed_node = self.to_skimmed_node(current_rotation_id, role, performance);

        SemiSkimmedNodeV1 {
            basic: skimmed_node,
            x25519_noise_versioned_key: self
                .description
                .host_information
                .keys
                .x25519_versioned_noise,
        }
    }

    pub fn to_semi_skimmed_node_v3(
        &self,
        current_rotation_id: u32,
        role: NodeRole,
        performance: Performance,
    ) -> SemiSkimmedNodeV3 {
        let skimmed_node = self.to_skimmed_node(current_rotation_id, role, performance);

        SemiSkimmedNodeV3 {
            basic: skimmed_node,
            noise_key: self
                .description
                .host_information
                .keys
                .x25519_versioned_noise,
            build_version: self.description.build_information.build_version.clone(),
            lp: self.description.lewes_protocol.clone(),
        }
    }
}

impl From<NymNodeDescriptionV3> for NymNodeDescriptionV2 {
    fn from(value: NymNodeDescriptionV3) -> Self {
        NymNodeDescriptionV2 {
            node_id: value.node_id,
            contract_node_type: value.contract_node_type,
            description: value.description.into(),
        }
    }
}

impl From<NymNodeDescriptionV3> for NymNodeDescriptionV1 {
    fn from(value: NymNodeDescriptionV3) -> Self {
        NymNodeDescriptionV1 {
            node_id: value.node_id,
            contract_node_type: value.contract_node_type,
            description: NymNodeDataV2::from(value.description).into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct NymNodeDataV3 {
    #[serde(default)]
    pub last_polled: OffsetDateTimeJsonSchemaWrapper,

    pub host_information: HostInformationV3,

    #[serde(default)]
    pub declared_role: DeclaredRolesV3,

    #[serde(default)]
    pub auxiliary_details: NymNodeAuxiliaryDetailsV3,

    // TODO: do we really care about ALL build info or just the version?
    pub build_information: BinaryBuildInformationOwned,

    #[serde(default)]
    pub network_requester: Option<NetworkRequesterDetailsV3>,

    #[serde(default)]
    pub ip_packet_router: Option<IpPacketRouterDetailsV3>,

    #[serde(default)]
    pub authenticator: Option<AuthenticatorDetailsV3>,

    #[serde(default)]
    pub wireguard: Option<WireguardDetailsV3>,

    // for now we only care about their ws/wss situation, nothing more
    pub mixnet_websockets: WebSocketsV3,

    #[serde(default)]
    pub lewes_protocol: Option<LewesProtocolDetailsV3>,
}

impl NymNodeDataV3 {
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

impl From<NymNodeDataV3> for NymNodeDataV2 {
    fn from(data: NymNodeDataV3) -> Self {
        NymNodeDataV2 {
            last_polled: data.last_polled,
            host_information: data.host_information,
            declared_role: data.declared_role,
            auxiliary_details: data.auxiliary_details.into(),
            build_information: data.build_information,
            network_requester: data.network_requester,
            ip_packet_router: data.ip_packet_router,
            authenticator: data.authenticator,
            wireguard: data.wireguard,
            mixnet_websockets: data.mixnet_websockets,
            lewes_protocol: data.lewes_protocol,
        }
    }
}

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, schemars::JsonSchema, ToSchema, PartialEq,
)]
pub struct NymNodeAuxiliaryDetailsV3 {
    /// Optional ISO 3166 alpha-2 two-letter country code of the node's **physical** location
    #[schema(example = "PL", value_type = Option<String>)]
    #[schemars(with = "Option<String>")]
    #[schemars(length(equal = 2))]
    pub location: Option<Country>,

    /// On-chain address of this node
    #[serde(default)]
    pub address: Option<String>,

    #[serde(default)]
    pub announce_ports: AnnouncePortsV3,

    /// Specifies whether this node operator has agreed to the terms and conditions
    /// as defined at <https://nymtech.net/terms-and-conditions/operators/v1.0.0>
    // make sure to include the default deserialisation as this field hasn't existed when the struct was first created
    #[serde(default)]
    pub accepted_operator_terms_and_conditions: bool,
}

impl From<NymNodeAuxiliaryDetailsV3> for NymNodeAuxiliaryDetailsV2 {
    fn from(value: NymNodeAuxiliaryDetailsV3) -> Self {
        NymNodeAuxiliaryDetailsV2 {
            location: value.location,
            announce_ports: value.announce_ports,
            accepted_operator_terms_and_conditions: value.accepted_operator_terms_and_conditions,
        }
    }
}

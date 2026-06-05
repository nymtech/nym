// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::described::type_translation::{
    AnnouncePortsV1, AuthenticatorDetailsV1, DeclaredRolesV1, HostInformationV1, HostKeysV1,
    IpPacketRouterDetailsV1, LewesProtocolDetailsV1, NetworkRequesterDetailsV1,
    NymNodeAuxiliaryDetailsV1, SphinxKeyV1, WebSocketsV1, WireguardDetailsV1,
};
use crate::models::described::v1::{DescribedNodeTypeV1, NymNodeDataV1, NymNodeDescriptionV1};
use crate::models::{BinaryBuildInformationOwned, OffsetDateTimeJsonSchemaWrapper};
use crate::nym_nodes::{BasicEntryInformation, NodeRole, SkimmedNodeV1};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_mixnet_contract_common::reward_params::Performance;
use nym_mixnet_contract_common::NodeId;
use nym_network_defaults::{DEFAULT_MIX_LISTENING_PORT, DEFAULT_VERLOC_LISTENING_PORT};
use nym_noise_keys::VersionedNoiseKeyV1;
use serde::{Deserialize, Serialize};
use tracing::warn;
use utoipa::ToSchema;

// no changes for the following types
pub type HostInformationV2 = HostInformationV1;
pub type DeclaredRolesV2 = DeclaredRolesV1;
pub type AnnouncePortsV2 = AnnouncePortsV1;
pub type NymNodeAuxiliaryDetailsV2 = NymNodeAuxiliaryDetailsV1;
pub type NetworkRequesterDetailsV2 = NetworkRequesterDetailsV1;
pub type IpPacketRouterDetailsV2 = IpPacketRouterDetailsV1;
pub type AuthenticatorDetailsV2 = AuthenticatorDetailsV1;
pub type WireguardDetailsV2 = WireguardDetailsV1;
pub type WebSocketsV2 = WebSocketsV1;
pub type DescribedNodeTypeV2 = DescribedNodeTypeV1;
pub type HostKeysV2 = HostKeysV1;
pub type SphinxKeyV2 = SphinxKeyV1;
pub type VersionedNoiseKeyV2 = VersionedNoiseKeyV1;

// to whoever is thinking of modifying this struct.
// you MUST NOT change its structure in any way - adding, removing or changing fields
// otherwise, it will break old clients as bincode serialisation is not backwards compatible
// even if you put `#[serde(default)]` all over the place
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct NymNodeDescriptionV2 {
    #[schema(value_type = u32)]
    pub node_id: NodeId,
    pub contract_node_type: DescribedNodeTypeV2,
    pub description: NymNodeDataV2,
}

impl NymNodeDescriptionV2 {
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
}

// to whoever is thinking of modifying this struct.
// you MUST NOT change its structure in any way - adding, removing or changing fields
// otherwise, it will break old clients as bincode serialisation is not backwards compatible
// even if you put `#[serde(default)]` all over the place
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct NymNodeDataV2 {
    #[serde(default)]
    pub last_polled: OffsetDateTimeJsonSchemaWrapper,

    pub host_information: HostInformationV2,

    #[serde(default)]
    pub declared_role: DeclaredRolesV2,

    #[serde(default)]
    pub auxiliary_details: NymNodeAuxiliaryDetailsV2,

    // TODO: do we really care about ALL build info or just the version?
    pub build_information: BinaryBuildInformationOwned,

    #[serde(default)]
    pub network_requester: Option<NetworkRequesterDetailsV2>,

    #[serde(default)]
    pub ip_packet_router: Option<IpPacketRouterDetailsV2>,

    #[serde(default)]
    pub authenticator: Option<AuthenticatorDetailsV2>,

    #[serde(default)]
    pub wireguard: Option<WireguardDetailsV2>,

    // for now we only care about their ws/wss situation, nothing more
    pub mixnet_websockets: WebSocketsV2,

    #[serde(default)]
    pub lewes_protocol: Option<LewesProtocolDetailsV1>,
}

impl NymNodeDataV2 {
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

impl From<NymNodeDataV2> for NymNodeDataV1 {
    fn from(data: NymNodeDataV2) -> Self {
        NymNodeDataV1 {
            last_polled: data.last_polled,
            host_information: data.host_information,
            declared_role: data.declared_role,
            auxiliary_details: data.auxiliary_details,
            build_information: data.build_information,
            network_requester: data.network_requester,
            ip_packet_router: data.ip_packet_router,
            authenticator: data.authenticator,
            wireguard: data.wireguard,
            mixnet_websockets: data.mixnet_websockets,
        }
    }
}

impl From<NymNodeDataV1> for NymNodeDataV2 {
    fn from(data: NymNodeDataV1) -> Self {
        NymNodeDataV2 {
            last_polled: data.last_polled,
            host_information: data.host_information,
            declared_role: data.declared_role,
            auxiliary_details: data.auxiliary_details,
            build_information: data.build_information,
            network_requester: data.network_requester,
            ip_packet_router: data.ip_packet_router,
            authenticator: data.authenticator,
            wireguard: data.wireguard,
            mixnet_websockets: data.mixnet_websockets,
            lewes_protocol: Default::default(),
        }
    }
}

impl From<NymNodeDescriptionV2> for NymNodeDescriptionV1 {
    fn from(value: NymNodeDescriptionV2) -> Self {
        NymNodeDescriptionV1 {
            node_id: value.node_id,
            contract_node_type: value.contract_node_type,
            description: value.description.into(),
        }
    }
}

impl From<NymNodeDescriptionV1> for NymNodeDescriptionV2 {
    fn from(value: NymNodeDescriptionV1) -> Self {
        NymNodeDescriptionV2 {
            node_id: value.node_id,
            contract_node_type: value.contract_node_type,
            description: value.description.into(),
        }
    }
}

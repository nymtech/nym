// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::{
    AuthenticatorDetailsV1, AuxiliaryDetailsV1, BinaryBuildInformationOwned, DeclaredRolesV1,
    DescribedNodeTypeV1, HostInformationV1, HostKeysV1, IpPacketRouterDetailsV1,
    LewesProtocolDetailsV1, NetworkRequesterDetailsV1, NymNodeDataV1, NymNodeDescriptionV1,
    OffsetDateTimeJsonSchemaWrapper, SphinxKeyV1, WebSocketsV1, WireguardDetailsV1,
};
use crate::nym_nodes::{BasicEntryInformation, NodeRole, SemiSkimmedNode, SkimmedNode};
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
pub type AuxiliaryDetailsV2 = AuxiliaryDetailsV1;
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
    pub auxiliary_details: AuxiliaryDetailsV2,

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

#[cfg(test)]
pub fn mock_nym_node_description(seed: u64) -> NymNodeDescriptionV2 {
    use crate::models::{LPHashFunction, LPSignatureScheme, LPKEM};
    use nym_test_utils::helpers::{u64_seeded_rng, RngCore};

    let mut rng = u64_seeded_rng(seed);

    let ed25519 = ed25519::KeyPair::new(&mut rng);

    // just reuse the same x25519 key for everything - this is just a data mock
    let x25519 = x25519::KeyPair::new(&mut rng);

    let mut kem_hashes_wrapper = std::collections::HashMap::new();
    let mut signing_keys_hashes_wrapper = std::collections::HashMap::new();
    let mut kem_hashes = std::collections::HashMap::new();
    let mut signing_keys_hashes = std::collections::HashMap::new();

    kem_hashes.insert(
        LPHashFunction::Sha256,
        hex::encode([(seed % 256) as u8; 32]),
    );
    kem_hashes_wrapper.insert(LPKEM::X25519, kem_hashes);

    signing_keys_hashes.insert(
        LPHashFunction::Sha256,
        hex::encode([(seed % 256) as u8; 32]),
    );
    signing_keys_hashes_wrapper.insert(LPSignatureScheme::Ed25519, signing_keys_hashes);

    NymNodeDescriptionV2 {
        node_id: rng.next_u32(),
        contract_node_type: DescribedNodeTypeV1::NymNode,
        description: NymNodeDataV2 {
            last_polled: time::OffsetDateTime::from_unix_timestamp(1767225600)
                .unwrap()
                .into(),
            host_information: HostInformationV2 {
                ip_address: vec![
                    std::net::IpAddr::V4(std::net::Ipv4Addr::new(1, 2, 3, (seed % 255) as u8)),
                ],
                hostname: Some(format!("my-awesome-node-{seed}.com")),
                keys: HostKeysV2 {
                    ed25519: *ed25519.public_key(),
                    x25519: *x25519.public_key(),
                    current_x25519_sphinx_key: SphinxKeyV2 {
                        rotation_id: 123,
                        public_key: *x25519.public_key(),
                    },
                    pre_announced_x25519_sphinx_key: None,
                    x25519_versioned_noise: Some(VersionedNoiseKeyV2 {
                        supported_version: nym_noise_keys::NoiseVersion::V1,
                        x25519_pubkey: *x25519.public_key(),
                    }),
                },
            },
            declared_role: DeclaredRolesV2 {
                mixnode: false,
                entry: true,
                exit_nr: true,
                exit_ipr: true,
            },
            auxiliary_details: AuxiliaryDetailsV2 {
                location: Some(celes::Country::switzerland()),
                announce_ports: Default::default(),
                accepted_operator_terms_and_conditions: true,
            },
            build_information: BinaryBuildInformationOwned {
                binary_name: "dummy-node".to_string(),
                build_timestamp: "2021-02-23T20:14:46.558472672+00:00".to_string(),
                build_version: "0.1.0-9-g46f83e1".to_string(),
                commit_sha: "46f83e112520533338245862d366f6a02cef07d4".to_string(),
                commit_timestamp: "2021-02-23T08:08:02-05:00".to_string(),
                commit_branch: "master".to_string(),
                rustc_version: "1.52.0-nightly".to_string(),
                rustc_channel: "nightly".to_string(),
                cargo_profile: "release".to_string(),
                cargo_triple: "wasm32-unknown-unknown".to_string(),
            },
            network_requester: Some(NetworkRequesterDetailsV2 {
                address: "FhtkzizQg2JbZ19kGkRKXdjV2QnFbT5ww88ZAKaD4nkF.7Remi4UVYzn1yL3qYtEcQBGh6tzTYxMdYB4uqyHVc5Z4@62F81C9GrHDRja9WCqozemRFSzFPMecY85MbGwn6efve".to_string(),
                uses_exit_policy: true,
            }),
            ip_packet_router: Some(IpPacketRouterDetailsV2 {
                address: "FhtkzizQg2JbZ19kGkRKXdjV2QnFbT5ww88ZAKaD4nkF.7Remi4UVYzn1yL3qYtEcQBGh6tzTYxMdYB4uqyHVc5Z4@62F81C9GrHDRja9WCqozemRFSzFPMecY85MbGwn6efve".to_string(),
            }),
            authenticator: Some(AuthenticatorDetailsV2 {
                address: "FhtkzizQg2JbZ19kGkRKXdjV2QnFbT5ww88ZAKaD4nkF.7Remi4UVYzn1yL3qYtEcQBGh6tzTYxMdYB4uqyHVc5Z4@62F81C9GrHDRja9WCqozemRFSzFPMecY85MbGwn6efve".to_string(),
            }),
            wireguard: Some(WireguardDetailsV2 {
                port: 123,
                tunnel_port: 234,
                metadata_port: 456,
                public_key: x25519.public_key().to_base58_string(),
            }),
            lewes_protocol: Some(LewesProtocolDetailsV1 {
                enabled: true,
                control_port: 1234,
                data_port: 2345,
                x25519: *x25519.public_key(),
                kem_keys: kem_hashes_wrapper,
                signing_keys: signing_keys_hashes_wrapper,
            }),
            mixnet_websockets: WebSocketsV2 {
                ws_port: 9000,
                wss_port: None,
            },
        },
    }
}

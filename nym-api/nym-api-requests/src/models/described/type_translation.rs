// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! This redefines relevant types present within nym-node-requests for the purposes of this crate
//! and defines required conversion methods

use celes::Country;
use nym_crypto::asymmetric::ed25519::serde_helpers::bs58_ed25519_pubkey;
use nym_crypto::asymmetric::x25519::serde_helpers::bs58_x25519_pubkey;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_network_defaults::{WG_METADATA_PORT, WG_TUNNEL_PORT};
use nym_noise_keys::VersionedNoiseKey;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use strum_macros::{Display, EnumString};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct HostInformation {
    #[schema(value_type = Vec<String>)]
    pub ip_address: Vec<IpAddr>,
    pub hostname: Option<String>,
    pub keys: HostKeys,
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

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct AnnouncePorts {
    pub verloc_port: Option<u16>,
    pub mix_port: Option<u16>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct AuxiliaryDetails {
    /// Optional ISO 3166 alpha-2 two-letter country code of the node's **physical** location
    #[schema(example = "PL", value_type = Option<String>)]
    #[schemars(with = "Option<String>")]
    #[schemars(length(equal = 2))]
    pub location: Option<Country>,

    #[serde(default)]
    pub announce_ports: AnnouncePorts,

    /// Specifies whether this node operator has agreed to the terms and conditions
    /// as defined at <https://nymtech.net/terms-and-conditions/operators/v1.0.0>
    // make sure to include the default deserialisation as this field hasn't existed when the struct was first created
    #[serde(default)]
    pub accepted_operator_terms_and_conditions: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct SphinxKey {
    pub rotation_id: u32,

    #[serde(with = "bs58_x25519_pubkey")]
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub public_key: x25519::PublicKey,
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

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct WebSockets {
    pub ws_port: u16,

    pub wss_port: Option<u16>,
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

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct AuthenticatorDetails {
    /// address of the embedded authenticator
    pub address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct WireguardDetails {
    // NOTE: the port field is deprecated in favour of tunnel_port
    pub port: u16,
    #[serde(default = "default_tunnel_port")]
    pub tunnel_port: u16,
    #[serde(default = "default_metadata_port")]
    pub metadata_port: u16,
    pub public_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct LewesProtocolDetails {
    /// Helper field that specifies whether the LP listener(s) is enabled on this node.
    /// It is directly controlled by the node's role (i.e. it is enabled if it supports 'entry' mode)
    pub enabled: bool,

    /// LP TCP control address (default: 41264) for establishing LP sessions
    pub control_port: u16,

    /// LP UDP data address (default: 51264) for Sphinx packets wrapped in LP
    pub data_port: u16,

    /// Digests of the KEM keys available to this node alongside hashing algorithms used
    /// for their computation.
    pub kem_keys: HashMap<LPKEM, HashMap<LPHashFunction, Vec<u8>>>,
}

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    JsonSchema,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Display,
    EnumString,
    ToSchema,
)]
#[strum(serialize_all = "lowercase")]
pub enum LPKEM {
    MlKem768,
    XWing,
    X25519,
    McEliece,
}

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    JsonSchema,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Display,
    EnumString,
    ToSchema,
)]
#[strum(serialize_all = "lowercase")]
pub enum LPHashFunction {
    Blake3,
    Shake128,
    Shake256,
    Sha256,
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

impl From<nym_node_requests::api::v1::node::models::AnnouncePorts> for AnnouncePorts {
    fn from(value: nym_node_requests::api::v1::node::models::AnnouncePorts) -> Self {
        AnnouncePorts {
            verloc_port: value.verloc_port,
            mix_port: value.mix_port,
        }
    }
}

impl From<nym_node_requests::api::v1::node::models::AuxiliaryDetails> for AuxiliaryDetails {
    fn from(value: nym_node_requests::api::v1::node::models::AuxiliaryDetails) -> Self {
        AuxiliaryDetails {
            location: value.location,
            announce_ports: value.announce_ports.into(),
            accepted_operator_terms_and_conditions: value.accepted_operator_terms_and_conditions,
        }
    }
}

impl From<nym_node_requests::api::v1::node::models::NodeRoles> for DeclaredRoles {
    fn from(value: nym_node_requests::api::v1::node::models::NodeRoles) -> Self {
        DeclaredRoles {
            mixnode: value.mixnode_enabled,
            entry: value.gateway_enabled,
            exit_nr: value.gateway_enabled && value.network_requester_enabled,
            exit_ipr: value.gateway_enabled && value.ip_packet_router_enabled,
        }
    }
}

impl From<nym_node_requests::api::v1::gateway::models::WebSockets> for WebSockets {
    fn from(value: nym_node_requests::api::v1::gateway::models::WebSockets) -> Self {
        WebSockets {
            ws_port: value.ws_port,
            wss_port: value.wss_port,
        }
    }
}

// works for current simple case.
impl From<nym_node_requests::api::v1::ip_packet_router::models::IpPacketRouter>
    for IpPacketRouterDetails
{
    fn from(value: nym_node_requests::api::v1::ip_packet_router::models::IpPacketRouter) -> Self {
        IpPacketRouterDetails {
            address: value.address,
        }
    }
}

// works for current simple case.
impl From<nym_node_requests::api::v1::authenticator::models::Authenticator>
    for AuthenticatorDetails
{
    fn from(value: nym_node_requests::api::v1::authenticator::models::Authenticator) -> Self {
        AuthenticatorDetails {
            address: value.address,
        }
    }
}

fn default_tunnel_port() -> u16 {
    WG_TUNNEL_PORT
}
fn default_metadata_port() -> u16 {
    WG_METADATA_PORT
}

// works for current simple case.
impl From<nym_node_requests::api::v1::gateway::models::Wireguard> for WireguardDetails {
    fn from(value: nym_node_requests::api::v1::gateway::models::Wireguard) -> Self {
        WireguardDetails {
            port: value.port,
            tunnel_port: value.tunnel_port,
            metadata_port: value.metadata_port,
            public_key: value.public_key,
        }
    }
}

impl From<nym_node_requests::api::v1::lewes_protocol::models::LewesProtocol>
    for LewesProtocolDetails
{
    fn from(value: nym_node_requests::api::v1::lewes_protocol::models::LewesProtocol) -> Self {
        LewesProtocolDetails {
            enabled: value.enabled,
            control_port: value.control_port,
            data_port: value.data_port,
            kem_keys: value
                .kem_keys
                .into_iter()
                .map(|(kem, digests)| {
                    (
                        kem.into(),
                        digests
                            .into_iter()
                            .map(|(hash_fn, digest)| (hash_fn.into(), digest))
                            .collect(),
                    )
                })
                .collect(),
        }
    }
}

impl From<nym_node_requests::api::v1::lewes_protocol::models::LPKEM> for LPKEM {
    fn from(value: nym_node_requests::api::v1::lewes_protocol::models::LPKEM) -> Self {
        match value {
            nym_node_requests::api::v1::lewes_protocol::models::LPKEM::MlKem768 => LPKEM::MlKem768,
            nym_node_requests::api::v1::lewes_protocol::models::LPKEM::XWing => LPKEM::XWing,
            nym_node_requests::api::v1::lewes_protocol::models::LPKEM::X25519 => LPKEM::X25519,
            nym_node_requests::api::v1::lewes_protocol::models::LPKEM::McEliece => LPKEM::McEliece,
        }
    }
}

impl From<nym_node_requests::api::v1::lewes_protocol::models::LPHashFunction> for LPHashFunction {
    fn from(value: nym_node_requests::api::v1::lewes_protocol::models::LPHashFunction) -> Self {
        match value {
            nym_node_requests::api::v1::lewes_protocol::models::LPHashFunction::Blake3 => {
                LPHashFunction::Blake3
            }
            nym_node_requests::api::v1::lewes_protocol::models::LPHashFunction::Shake128 => {
                LPHashFunction::Shake128
            }
            nym_node_requests::api::v1::lewes_protocol::models::LPHashFunction::Shake256 => {
                LPHashFunction::Shake256
            }
            nym_node_requests::api::v1::lewes_protocol::models::LPHashFunction::Sha256 => {
                LPHashFunction::Sha256
            }
        }
    }
}

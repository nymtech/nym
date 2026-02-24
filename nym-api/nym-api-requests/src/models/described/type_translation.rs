// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! This redefines relevant types present within nym-node-requests for the purposes of this crate
//! and defines required conversion methods

use celes::Country;
use nym_crypto::asymmetric::ed25519::serde_helpers::bs58_ed25519_pubkey;
use nym_crypto::asymmetric::x25519::serde_helpers::{bs58_dh_public_key, bs58_x25519_pubkey};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_kkt_ciphersuite::{HashFunction, SignatureScheme, KEM};
use nym_network_defaults::{WG_METADATA_PORT, WG_TUNNEL_PORT};
use nym_noise_keys::VersionedNoiseKeyV1;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use strum_macros::{Display, EnumString};
use thiserror::Error;
use utoipa::ToSchema;

// to whoever is thinking of modifying this struct.
// you MUST NOT change its structure in any way - adding, removing or changing fields
// otherwise, it will break old clients as bincode serialisation is not backwards compatible
// even if you put `#[serde(default)]` all over the place
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema, PartialEq)]
pub struct HostInformationV1 {
    #[schema(value_type = Vec<String>)]
    pub ip_address: Vec<IpAddr>,
    pub hostname: Option<String>,
    pub keys: HostKeysV1,
}

// to whoever is thinking of modifying this struct.
// you MUST NOT change its structure in any way - adding, removing or changing fields
// otherwise, it will break old clients as bincode serialisation is not backwards compatible
// even if you put `#[serde(default)]` all over the place
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema, PartialEq)]
pub struct HostKeysV1 {
    #[serde(with = "bs58_ed25519_pubkey")]
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub ed25519: ed25519::PublicKey,

    #[deprecated(note = "use the current_x25519_sphinx_key with explicit rotation information")]
    #[serde(with = "bs58_x25519_pubkey")]
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub x25519: x25519::PublicKey,

    pub current_x25519_sphinx_key: SphinxKeyV1,

    #[serde(default)]
    pub pre_announced_x25519_sphinx_key: Option<SphinxKeyV1>,

    #[serde(default)]
    pub x25519_versioned_noise: Option<VersionedNoiseKeyV1>,
}

// to whoever is thinking of modifying this struct.
// you MUST NOT change its structure in any way - adding, removing or changing fields
// otherwise, it will break old clients as bincode serialisation is not backwards compatible
// even if you put `#[serde(default)]` all over the place
#[derive(
    Clone, Copy, Debug, Default, Serialize, Deserialize, schemars::JsonSchema, ToSchema, PartialEq,
)]
pub struct AnnouncePortsV1 {
    pub verloc_port: Option<u16>,
    pub mix_port: Option<u16>,
}

// to whoever is thinking of modifying this struct.
// you MUST NOT change its structure in any way - adding, removing or changing fields
// otherwise, it will break old clients as bincode serialisation is not backwards compatible
// even if you put `#[serde(default)]` all over the place
#[derive(
    Clone, Copy, Debug, Default, Serialize, Deserialize, schemars::JsonSchema, ToSchema, PartialEq,
)]
pub struct AuxiliaryDetailsV1 {
    /// Optional ISO 3166 alpha-2 two-letter country code of the node's **physical** location
    #[schema(example = "PL", value_type = Option<String>)]
    #[schemars(with = "Option<String>")]
    #[schemars(length(equal = 2))]
    pub location: Option<Country>,

    #[serde(default)]
    pub announce_ports: AnnouncePortsV1,

    /// Specifies whether this node operator has agreed to the terms and conditions
    /// as defined at <https://nymtech.net/terms-and-conditions/operators/v1.0.0>
    // make sure to include the default deserialisation as this field hasn't existed when the struct was first created
    #[serde(default)]
    pub accepted_operator_terms_and_conditions: bool,
}

// to whoever is thinking of modifying this struct.
// you MUST NOT change its structure in any way - adding, removing or changing fields
// otherwise, it will break old clients as bincode serialisation is not backwards compatible
// even if you put `#[serde(default)]` all over the place
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema, PartialEq)]
pub struct SphinxKeyV1 {
    pub rotation_id: u32,

    #[serde(with = "bs58_x25519_pubkey")]
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    pub public_key: x25519::PublicKey,
}

// to whoever is thinking of modifying this struct.
// you MUST NOT change its structure in any way - adding, removing or changing fields
// otherwise, it will break old clients as bincode serialisation is not backwards compatible
// even if you put `#[serde(default)]` all over the place
#[derive(
    Clone, Copy, Debug, Default, Serialize, Deserialize, schemars::JsonSchema, ToSchema, PartialEq,
)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(
        export,
        export_to = "ts-packages/types/src/types/rust/DeclaredRoles.ts"
    )
)]
pub struct DeclaredRolesV1 {
    pub mixnode: bool,
    pub entry: bool,
    pub exit_nr: bool,
    pub exit_ipr: bool,
}

impl DeclaredRolesV1 {
    pub fn can_operate_exit_gateway(&self) -> bool {
        self.exit_ipr && self.exit_nr
    }
}

// to whoever is thinking of modifying this struct.
// you MUST NOT change its structure in any way - adding, removing or changing fields
// otherwise, it will break old clients as bincode serialisation is not backwards compatible
// even if you put `#[serde(default)]` all over the place
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema, PartialEq)]
pub struct WebSocketsV1 {
    pub ws_port: u16,

    pub wss_port: Option<u16>,
}

// to whoever is thinking of modifying this struct.
// you MUST NOT change its structure in any way - adding, removing or changing fields
// otherwise, it will break old clients as bincode serialisation is not backwards compatible
// even if you put `#[serde(default)]` all over the place
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema, PartialEq)]
pub struct NetworkRequesterDetailsV1 {
    /// address of the embedded network requester
    pub address: String,

    /// flag indicating whether this network requester uses the exit policy rather than the deprecated allow list
    pub uses_exit_policy: bool,
}

// to whoever is thinking of modifying this struct.
// you MUST NOT change its structure in any way - adding, removing or changing fields
// otherwise, it will break old clients as bincode serialisation is not backwards compatible
// even if you put `#[serde(default)]` all over the place
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema, PartialEq)]
pub struct IpPacketRouterDetailsV1 {
    /// address of the embedded ip packet router
    pub address: String,
}

// to whoever is thinking of modifying this struct.
// you MUST NOT change its structure in any way - adding, removing or changing fields
// otherwise, it will break old clients as bincode serialisation is not backwards compatible
// even if you put `#[serde(default)]` all over the place
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema, PartialEq)]
pub struct AuthenticatorDetailsV1 {
    /// address of the embedded authenticator
    pub address: String,
}

// to whoever is thinking of modifying this struct.
// you MUST NOT change its structure in any way - adding, removing or changing fields
// otherwise, it will break old clients as bincode serialisation is not backwards compatible
// even if you put `#[serde(default)]` all over the place
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema, PartialEq)]
pub struct WireguardDetailsV1 {
    // NOTE: the port field is deprecated in favour of tunnel_port
    pub port: u16,
    #[serde(default = "default_tunnel_port")]
    pub tunnel_port: u16,
    #[serde(default = "default_metadata_port")]
    pub metadata_port: u16,
    pub public_key: String,
}

// to whoever is thinking of modifying this struct.
// you MUST NOT change its structure in any way - adding, removing or changing fields
// otherwise, it will break old clients as bincode serialisation is not backwards compatible
// even if you put `#[serde(default)]` all over the place
#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema, PartialEq)]
pub struct LewesProtocolDetailsV1 {
    /// Helper field that specifies whether the LP listener(s) is enabled on this node.
    /// It is directly controlled by the node's role (i.e. it is enabled if it supports 'entry' mode)
    pub enabled: bool,

    /// LP TCP control address (default: 41264) for establishing LP sessions
    pub control_port: u16,

    /// LP UDP data address (default: 51264) for Sphinx packets wrapped in LP
    pub data_port: u16,

    #[serde(with = "bs58_dh_public_key")]
    #[schemars(with = "String")]
    #[schema(value_type = String)]
    /// LP public key
    pub x25519: x25519::DHPublicKey,

    /// Digests of the KEM keys available to this node alongside hashing algorithms used
    /// for their computation.
    /// note: digests are hex encoded
    pub kem_keys: HashMap<LPKEM, HashMap<LPHashFunction, String>>,
}

impl LewesProtocolDetailsV1 {
    fn decode_digests(
        digests: &HashMap<LPHashFunction, String>,
    ) -> Result<HashMap<HashFunction, Vec<u8>>, MalformedLPData> {
        let mut kem_digests = HashMap::new();
        for (hash_function, digest) in digests {
            let digest = hex::decode(digest).map_err(|source| MalformedLPData::MalformedHash {
                value: digest.to_string(),
                source,
            })?;
            kem_digests.insert((*hash_function).try_into()?, digest);
        }
        Ok(kem_digests)
    }

    pub fn kem_keys(
        &self,
    ) -> Result<HashMap<KEM, HashMap<HashFunction, Vec<u8>>>, MalformedLPData> {
        let mut kem_keys = HashMap::new();
        for (kem, digests) in &self.kem_keys {
            let kem_digests = Self::decode_digests(digests)?;
            kem_keys.insert((*kem).try_into()?, kem_digests);
        }
        Ok(kem_keys)
    }
}

/// Convert map of digests from `nym_node_requests` types into `nym-api-requests` types
fn translate_digests(
    digests: HashMap<nym_node_requests::api::v1::lewes_protocol::models::LPHashFunction, String>,
) -> HashMap<LPHashFunction, String> {
    digests
        .into_iter()
        .map(|(hash_fn, digest)| (hash_fn.into(), digest))
        .collect()
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
#[non_exhaustive]
pub enum LPKEM {
    MlKem768,
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
#[non_exhaustive]
pub enum LPHashFunction {
    Blake3,
    Shake128,
    Shake256,
    Sha256,
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
#[non_exhaustive]
pub enum LPSignatureScheme {
    Ed25519,
}

impl From<nym_node_requests::api::v1::node::models::HostInformation> for HostInformationV1 {
    fn from(value: nym_node_requests::api::v1::node::models::HostInformation) -> Self {
        HostInformationV1 {
            ip_address: value.ip_address,
            hostname: value.hostname,
            keys: value.keys.into(),
        }
    }
}

impl From<nym_node_requests::api::v1::node::models::SphinxKey> for SphinxKeyV1 {
    fn from(value: nym_node_requests::api::v1::node::models::SphinxKey) -> Self {
        SphinxKeyV1 {
            rotation_id: value.rotation_id,
            public_key: value.public_key,
        }
    }
}

impl From<nym_node_requests::api::v1::node::models::HostKeys> for HostKeysV1 {
    fn from(value: nym_node_requests::api::v1::node::models::HostKeys) -> Self {
        HostKeysV1 {
            ed25519: value.ed25519_identity,
            x25519: value.x25519_sphinx,
            current_x25519_sphinx_key: value.primary_x25519_sphinx_key.into(),
            pre_announced_x25519_sphinx_key: value.pre_announced_x25519_sphinx_key.map(Into::into),
            x25519_versioned_noise: value.x25519_versioned_noise,
        }
    }
}

impl From<nym_node_requests::api::v1::node::models::AnnouncePorts> for AnnouncePortsV1 {
    fn from(value: nym_node_requests::api::v1::node::models::AnnouncePorts) -> Self {
        AnnouncePortsV1 {
            verloc_port: value.verloc_port,
            mix_port: value.mix_port,
        }
    }
}

impl From<nym_node_requests::api::v1::node::models::AuxiliaryDetails> for AuxiliaryDetailsV1 {
    fn from(value: nym_node_requests::api::v1::node::models::AuxiliaryDetails) -> Self {
        AuxiliaryDetailsV1 {
            location: value.location,
            announce_ports: value.announce_ports.into(),
            accepted_operator_terms_and_conditions: value.accepted_operator_terms_and_conditions,
        }
    }
}

impl From<nym_node_requests::api::v1::node::models::NodeRoles> for DeclaredRolesV1 {
    fn from(value: nym_node_requests::api::v1::node::models::NodeRoles) -> Self {
        DeclaredRolesV1 {
            mixnode: value.mixnode_enabled,
            entry: value.gateway_enabled,
            exit_nr: value.gateway_enabled && value.network_requester_enabled,
            exit_ipr: value.gateway_enabled && value.ip_packet_router_enabled,
        }
    }
}

impl From<nym_node_requests::api::v1::gateway::models::WebSockets> for WebSocketsV1 {
    fn from(value: nym_node_requests::api::v1::gateway::models::WebSockets) -> Self {
        WebSocketsV1 {
            ws_port: value.ws_port,
            wss_port: value.wss_port,
        }
    }
}

// works for current simple case.
impl From<nym_node_requests::api::v1::ip_packet_router::models::IpPacketRouter>
    for IpPacketRouterDetailsV1
{
    fn from(value: nym_node_requests::api::v1::ip_packet_router::models::IpPacketRouter) -> Self {
        IpPacketRouterDetailsV1 {
            address: value.address,
        }
    }
}

// works for current simple case.
impl From<nym_node_requests::api::v1::authenticator::models::Authenticator>
    for AuthenticatorDetailsV1
{
    fn from(value: nym_node_requests::api::v1::authenticator::models::Authenticator) -> Self {
        AuthenticatorDetailsV1 {
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
impl From<nym_node_requests::api::v1::gateway::models::Wireguard> for WireguardDetailsV1 {
    fn from(value: nym_node_requests::api::v1::gateway::models::Wireguard) -> Self {
        WireguardDetailsV1 {
            port: value.port,
            tunnel_port: value.tunnel_port,
            metadata_port: value.metadata_port,
            public_key: value.public_key,
        }
    }
}

impl From<nym_node_requests::api::v1::lewes_protocol::models::LewesProtocol>
    for LewesProtocolDetailsV1
{
    fn from(value: nym_node_requests::api::v1::lewes_protocol::models::LewesProtocol) -> Self {
        LewesProtocolDetailsV1 {
            enabled: value.enabled,
            control_port: value.control_port,
            data_port: value.data_port,
            x25519: value.x25519,
            kem_keys: value
                .kem_keys
                .into_iter()
                .map(|(kem, digests)| (kem.into(), translate_digests(digests)))
                .collect(),
        }
    }
}

impl From<nym_node_requests::api::v1::lewes_protocol::models::LPKEM> for LPKEM {
    fn from(value: nym_node_requests::api::v1::lewes_protocol::models::LPKEM) -> Self {
        match value {
            nym_node_requests::api::v1::lewes_protocol::models::LPKEM::MlKem768 => LPKEM::MlKem768,
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

impl From<nym_node_requests::api::v1::lewes_protocol::models::LPSignatureScheme>
    for LPSignatureScheme
{
    fn from(value: nym_node_requests::api::v1::lewes_protocol::models::LPSignatureScheme) -> Self {
        match value {
            nym_node_requests::api::v1::lewes_protocol::models::LPSignatureScheme::Ed25519 => {
                LPSignatureScheme::Ed25519
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum MalformedLPData {
    #[error("{value} does not correspond to any valid LP KEM")]
    UnknownLpKEM { value: LPKEM },

    #[error("{value} does not correspond to any valid LP Signature Scheme")]
    UnknownLpSignatureScheme { value: LPSignatureScheme },

    #[error("{value} does not correspond to any valid LP Hash Function")]
    UnknownLpHashFunction { value: LPHashFunction },

    #[error("{value} is not a valid hex encoded hash: {source}")]
    MalformedHash {
        value: String,
        source: hex::FromHexError,
    },
}

impl TryFrom<LPKEM> for KEM {
    type Error = MalformedLPData;
    fn try_from(value: LPKEM) -> Result<Self, Self::Error> {
        match value {
            LPKEM::MlKem768 => Ok(KEM::MlKem768),
            LPKEM::McEliece => Ok(KEM::McEliece),
            // TODO: for backwards compatibility once variants within the LP crate change
            // other => Err(MalformedLPData::UnknownLpKEM { value: other }),
        }
    }
}

impl TryFrom<LPHashFunction> for HashFunction {
    type Error = MalformedLPData;
    fn try_from(value: LPHashFunction) -> Result<Self, Self::Error> {
        match value {
            LPHashFunction::Blake3 => Ok(HashFunction::Blake3),
            LPHashFunction::Shake128 => Ok(HashFunction::Shake128),
            LPHashFunction::Shake256 => Ok(HashFunction::Shake256),
            LPHashFunction::Sha256 => Ok(HashFunction::SHA256),
            // TODO: for backwards compatibility once variants within the LP crate change
            // other => Err(MalformedLPData::UnknownLpHashFunction { value: other }),
        }
    }
}

impl TryFrom<LPSignatureScheme> for SignatureScheme {
    type Error = MalformedLPData;
    fn try_from(value: LPSignatureScheme) -> Result<Self, Self::Error> {
        match value {
            LPSignatureScheme::Ed25519 => Ok(SignatureScheme::Ed25519),
            // TODO: for backwards compatibility once variants within the LP crate change
            // other => Err(MalformedLPData::UnknownLpSignatureScheme { value: other }),
        }
    }
}

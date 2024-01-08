// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

pub use crate::api::SignedHostInformation;
pub use nym_bin_common::build_information::BinaryBuildInformationOwned;

#[derive(Clone, Default, Debug, Copy, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NodeRoles {
    pub mixnode_enabled: bool,
    pub gateway_enabled: bool,
    pub network_requester_enabled: bool,
    pub ip_packet_router_enabled: bool,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct HostInformation {
    /// Ip address(es) of this host, such as `1.1.1.1`.
    #[cfg_attr(feature = "openapi", schema(value_type = Vec<String>, format = Byte, example = json!(["1.1.1.1"])))]
    pub ip_address: Vec<IpAddr>,

    /// Optional hostname of this node, for example `nymtech.net`.
    #[cfg_attr(feature = "openapi", schema(example = "nymtech.net"))]
    pub hostname: Option<String>,

    /// Public keys associated with this node.
    pub keys: HostKeys,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct HostKeys {
    /// Base58-encoded ed25519 public key of this node. Currently it corresponds to either mixnode's or gateway's identity.
    pub ed25519: String,

    /// Base58-encoded x25519 public key of this node used for sphinx/outfox packet creation.
    /// Currently it corresponds to either mixnode's or gateway's key.
    pub x25519: String,
}

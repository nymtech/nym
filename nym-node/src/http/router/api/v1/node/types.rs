// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) use nym_bin_common::build_information::BinaryBuildInformationOwned;
use serde::Serialize;
use std::net::IpAddr;
use utoipa::ToSchema;

#[derive(Clone, Default, Debug, Copy, ToSchema, Serialize)]
pub struct NodeRoles {
    pub mixnode_enabled: bool,
    pub gateway_enabled: bool,
    pub network_requester_enabled: bool,
}

#[derive(Clone, Default, Debug, ToSchema, Serialize)]
pub struct HostInformation {
    /// Ip address(es) of this host, such as `1.1.1.1`.
    pub ip_address: Vec<IpAddr>,

    /// Optional hostname of this node, for example `nymtech.net`.
    pub hostname: Option<String>,

    /// Public keys associated with this node.
    pub keys: HostKeys,
}

#[derive(Clone, Default, Debug, ToSchema, Serialize)]
pub struct HostKeys {
    /// Base58-encoded ed25519 public key of this node. Currently it corresponds to either mixnode's or gateway's identity.
    pub ed25519: String,

    /// Base58-encoded x25519 public key of this node used for sphinx/outfox packet creation.
    /// Currently it corresponds to either mixnode's or gateway's key.
    pub x25519: String,
}

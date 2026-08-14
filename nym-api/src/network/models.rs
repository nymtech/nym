// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_network_defaults::v2::NymNetworkDetails as NymNetworkDetailsV2;
use nym_network_defaults::NymNetworkDetails;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct NetworkDetails {
    pub(crate) connected_nyxd: String,
    pub(crate) network: NymNetworkDetails,
}

impl NetworkDetails {
    pub fn new(connected_nyxd: String, network: NymNetworkDetails) -> Self {
        Self {
            connected_nyxd,
            network,
        }
    }
}

/// Same shape as [`NetworkDetails`], but carries the v2 (grouped `networking` block)
/// version of the network details struct. This is *not* a v2 of the API - it's the
/// existing `/v1/network` API surface serving the newer struct shape alongside the
/// original one.
#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct NetworkDetailsV2 {
    pub(crate) connected_nyxd: String,
    pub(crate) network: NymNetworkDetailsV2,
}

impl NetworkDetailsV2 {
    #[allow(unused)]
    pub fn new(connected_nyxd: String, network: NymNetworkDetailsV2) -> Self {
        Self {
            connected_nyxd,
            network,
        }
    }
}

/// Converts down to the v1 shape for the `/v1/network/details` handler. Loses whatever
/// `network.networking.dns_fallbacks` carries, since v1 has nowhere to put it.
impl From<NetworkDetailsV2> for NetworkDetails {
    fn from(v2: NetworkDetailsV2) -> Self {
        NetworkDetails {
            connected_nyxd: v2.connected_nyxd,
            network: v2.network.into(),
        }
    }
}

/// Converts up to the v2 shape, deriving `network.networking` from the v1 struct's
/// `nym_api_urls()` / `nym_vpn_api_urls()` accessors (`dns_fallbacks` starts out empty).
impl From<NetworkDetails> for NetworkDetailsV2 {
    fn from(v1: NetworkDetails) -> Self {
        NetworkDetailsV2 {
            connected_nyxd: v1.connected_nyxd,
            network: v1.network.into(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ContractInformation<T> {
    pub(crate) address: Option<String>,
    pub(crate) details: Option<T>,
}

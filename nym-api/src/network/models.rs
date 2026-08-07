// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_config::defaults::v2::NymNetworkDetails as NymNetworkDetailsV2;
use nym_config::defaults::NymNetworkDetails;
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
    pub fn new(connected_nyxd: String, network: NymNetworkDetailsV2) -> Self {
        Self {
            connected_nyxd,
            network,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ContractInformation<T> {
    pub(crate) address: Option<String>,
    pub(crate) details: Option<T>,
}

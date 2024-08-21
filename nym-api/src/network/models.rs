// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

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

#[derive(Serialize, Deserialize, Clone, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct ContractInformation<T> {
    pub(crate) address: Option<String>,
    pub(crate) details: Option<T>,
}

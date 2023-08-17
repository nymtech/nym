// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_config::defaults::NymNetworkDetails;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
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

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ContractInformation<T> {
    pub(crate) address: Option<String>,
    pub(crate) details: Option<T>,
}

// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct NodeHealth {
    pub status: NodeStatus,
    pub uptime: u64,
}

impl NodeHealth {
    pub fn new_healthy(uptime: Duration) -> Self {
        NodeHealth {
            status: NodeStatus::Up,
            uptime: uptime.as_secs(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub enum NodeStatus {
    Up,
}

impl NodeStatus {
    pub fn is_up(&self) -> bool {
        matches!(self, NodeStatus::Up)
    }
}

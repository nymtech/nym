// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_mixnet_contract_common::{GatewayBond, MixNodeDetails, NodeId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct LegacyGatewayBondWithId {
    #[serde(flatten)]
    pub bond: GatewayBond,
    pub node_id: NodeId,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct LegacyMixnodesResponse {
    pub count: usize,
    pub nodes: Vec<MixNodeDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct LegacyGatewaysResponse {
    pub count: usize,
    pub nodes: Vec<LegacyGatewayBondWithId>,
}

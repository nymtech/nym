// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Decimal;
use mixnet_contract_common::mixnode::MixNodeDetails;
use mixnet_contract_common::reward_params::{Performance, RewardingParams};
use mixnet_contract_common::{MixNode, NodeId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/MixnodeStatus.ts")
)]
#[serde(rename_all = "snake_case")]
pub enum MixnodeStatus {
    Active,   // in both the active set and the rewarded set
    Standby,  // only in the rewarded set
    Inactive, // in neither the rewarded set nor the active set, but is bonded
    NotFound, // doesn't even exist in the bonded set
}

impl MixnodeStatus {
    pub fn is_active(&self) -> bool {
        *self == MixnodeStatus::Active
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/CoreNodeStatusResponse.ts")
)]
pub struct CoreNodeStatusResponse {
    pub identity: String,
    pub count: i32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/MixnodeStatusResponse.ts")
)]
pub struct MixnodeStatusResponse {
    pub status: MixnodeStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct MixNodeBondAnnotated {
    pub mixnode_details: MixNodeDetails,
    pub stake_saturation: StakeSaturation,
    pub performance: Performance,
    pub estimated_operator_apy: Decimal,
    pub estimated_delegators_apy: Decimal,
}

impl MixNodeBondAnnotated {
    pub fn mix_node(&self) -> &MixNode {
        &self.mixnode_details.bond_information.mix_node
    }

    pub fn mix_id(&self) -> NodeId {
        self.mixnode_details.mix_id()
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
pub struct RewardEstimationResponse {
    pub estimated_total_node_reward: u64,
    pub estimated_operator_reward: u64,
    pub estimated_delegators_reward: u64,
    pub estimated_node_profit: u64,
    pub estimated_operator_cost: u64,

    pub reward_params: RewardingParams,
    pub as_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct UptimeResponse {
    pub identity: String,
    pub avg_uptime: u8,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/StakeSaturationResponse.ts")
)]
pub struct StakeSaturationResponse {
    pub saturation: StakeSaturation,
    pub as_at: i64,
}

pub type StakeSaturation = Decimal;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/SelectionChance.ts")
)]
pub enum SelectionChance {
    VeryHigh,
    Moderate,
    Low,
}

impl From<f64> for SelectionChance {
    fn from(p: f64) -> SelectionChance {
        match p {
            p if p > 0.15 => SelectionChance::VeryHigh,
            p if p >= 0.05 => SelectionChance::Moderate,
            _ => SelectionChance::Low,
        }
    }
}

impl fmt::Display for SelectionChance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectionChance::VeryHigh => write!(f, "VeryHigh"),
            SelectionChance::Moderate => write!(f, "Moderate"),
            SelectionChance::Low => write!(f, "Low"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/InclusionProbabilityResponse.ts")
)]
pub struct InclusionProbabilityResponse {
    pub in_active: SelectionChance,
    pub in_reserve: SelectionChance,
}

impl fmt::Display for InclusionProbabilityResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "in_active: {}, in_reserve: {}",
            self.in_active, self.in_reserve
        )
    }
}

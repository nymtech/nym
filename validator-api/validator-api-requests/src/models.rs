// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Decimal;
use mixnet_contract_common::mixnode::MixNodeDetails;
use mixnet_contract_common::reward_params::{Performance, RewardingParams};
use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;
use mixnet_contract_common::rewarding::RewardEstimate;
use mixnet_contract_common::{Interval, MixNode, NodeId, RewardedSetNodeStatus};
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

impl From<MixnodeStatus> for Option<RewardedSetNodeStatus> {
    fn from(status: MixnodeStatus) -> Self {
        match status {
            MixnodeStatus::Active => Some(RewardedSetNodeStatus::Active),
            MixnodeStatus::Standby => Some(RewardedSetNodeStatus::Standby),
            MixnodeStatus::Inactive => None,
            MixnodeStatus::NotFound => None,
        }
    }
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
    ts(export_to = "ts-packages/types/src/types/rust/MixnodeCoreStatusResponse.ts")
)]
pub struct MixnodeCoreStatusResponse {
    pub mix_id: NodeId,
    pub count: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/GatewayCoreStatusResponse.ts")
)]
pub struct GatewayCoreStatusResponse {
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
    pub uncapped_stake_saturation: StakeSaturation,
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

#[derive(Deserialize, JsonSchema)]
pub struct ComputeRewardEstParam {
    pub performance: Option<Performance>,
    pub active_in_rewarded_set: Option<bool>,
    pub pledge_amount: Option<u64>,
    pub total_delegation: Option<u64>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
pub struct RewardEstimationResponse {
    pub estimation: RewardEstimate,

    pub reward_params: RewardingParams,
    pub epoch: Interval,
    pub as_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct UptimeResponse {
    pub mix_id: NodeId,
    pub avg_uptime: u8,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/StakeSaturationResponse.ts")
)]
pub struct StakeSaturationResponse {
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub saturation: StakeSaturation,

    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub uncapped_saturation: StakeSaturation,
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

impl From<Decimal> for SelectionChance {
    fn from(p: Decimal) -> Self {
        match p {
            p if p >= Decimal::from_ratio(15u32, 100u32) => SelectionChance::VeryHigh,
            p if p > Decimal::from_ratio(5u32, 100u32) => SelectionChance::Moderate,
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

// deprecated

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct DeprecatedUptimeResponse {
    pub identity: String,
    pub avg_uptime: u8,
    pub deprecated: bool,
}

#[derive(Deserialize, JsonSchema)]
pub struct DeprecatedComputeRewardEstParam {
    pub uptime: Option<u8>,
    pub is_active: Option<bool>,
    pub pledge_amount: Option<u64>,
    pub total_delegation: Option<u64>,
    pub deprecated: bool,
}

impl From<DeprecatedComputeRewardEstParam> for ComputeRewardEstParam {
    fn from(deprecated_params: DeprecatedComputeRewardEstParam) -> Self {
        ComputeRewardEstParam {
            performance: deprecated_params
                .uptime
                .map(|u| Performance::from_percentage_value(u as u64).unwrap_or_default()),
            active_in_rewarded_set: deprecated_params.is_active,
            pledge_amount: deprecated_params.pledge_amount,
            total_delegation: deprecated_params.total_delegation,
        }
    }
}

impl From<ComputeRewardEstParam> for DeprecatedComputeRewardEstParam {
    fn from(new_params: ComputeRewardEstParam) -> Self {
        DeprecatedComputeRewardEstParam {
            uptime: new_params.performance.map(|p| p.round_to_integer()),
            is_active: new_params.active_in_rewarded_set,
            pledge_amount: new_params.pledge_amount,
            total_delegation: new_params.total_delegation,
            deprecated: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
pub struct DeprecatedRewardEstimationResponse {
    pub estimated_total_node_reward: u64,
    pub estimated_operator_reward: u64,
    pub estimated_delegators_reward: u64,
    pub estimated_node_profit: u64,
    pub estimated_operator_cost: u64,

    pub reward_params: RewardingParams,
    pub as_at: i64,
    pub deprecated: bool,
}

impl From<RewardEstimationResponse> for DeprecatedRewardEstimationResponse {
    fn from(new_estimation: RewardEstimationResponse) -> Self {
        DeprecatedRewardEstimationResponse {
            estimated_total_node_reward: truncate_reward_amount(
                new_estimation.estimation.total_node_reward,
            )
            .u128()
            .try_into()
            .unwrap_or_default(),
            estimated_operator_reward: truncate_reward_amount(new_estimation.estimation.operator)
                .u128()
                .try_into()
                .unwrap_or_default(),
            estimated_delegators_reward: truncate_reward_amount(
                new_estimation.estimation.delegates,
            )
            .u128()
            .try_into()
            .unwrap_or_default(),
            estimated_node_profit: if new_estimation.estimation.operator
                < new_estimation.estimation.operating_cost
            {
                0
            } else {
                truncate_reward_amount(
                    new_estimation.estimation.operator - new_estimation.estimation.operating_cost,
                )
                .u128()
                .try_into()
                .unwrap_or_default()
            },
            estimated_operator_cost: truncate_reward_amount(
                new_estimation.estimation.operating_cost,
            )
            .u128()
            .try_into()
            .unwrap_or_default(),
            reward_params: new_estimation.reward_params,
            as_at: new_estimation.as_at,
            deprecated: true,
        }
    }
}

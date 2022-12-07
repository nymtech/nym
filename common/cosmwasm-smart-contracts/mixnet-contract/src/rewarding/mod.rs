// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Coin, Decimal};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod helpers;
pub mod simulator;

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/RewardEstimate.ts")
)]
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct RewardEstimate {
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub total_node_reward: Decimal,

    // note that operator reward includes the operating_cost,
    // i.e. say total_node_reward was `1nym` and operating_cost was `2nym`
    // in that case the operator reward would still be `1nym` as opposed to 0
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub operator: Decimal,
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub delegates: Decimal,
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub operating_cost: Decimal,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct RewardDistribution {
    pub operator: Decimal,
    pub delegates: Decimal,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct PendingRewardResponse {
    pub amount_staked: Option<Coin>,
    pub amount_earned: Option<Coin>,
    pub amount_earned_detailed: Option<Decimal>,

    /// The associated mixnode is still fully bonded, meaning it is neither unbonded
    /// nor in the process of unbonding that would have finished at the epoch transition.
    pub mixnode_still_fully_bonded: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
pub struct EstimatedCurrentEpochRewardResponse {
    pub original_stake: Option<Coin>,

    pub current_stake_value: Option<Coin>,
    pub current_stake_value_detailed_amount: Option<Decimal>,

    pub estimation: Option<Coin>,
    pub detailed_estimation_amount: Option<Decimal>,
}

impl EstimatedCurrentEpochRewardResponse {
    pub fn empty_response() -> Self {
        EstimatedCurrentEpochRewardResponse {
            original_stake: None,
            current_stake_value: None,
            current_stake_value_detailed_amount: None,
            estimation: None,
            detailed_estimation_amount: None,
        }
    }
}

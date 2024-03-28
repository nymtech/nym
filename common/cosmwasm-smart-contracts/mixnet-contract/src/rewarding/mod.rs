// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal};

pub mod helpers;
pub mod simulator;

#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "ts-packages/types/src/types/rust/RewardEstimate.ts")
)]
#[cw_serde]
#[derive(Copy, Default)]
pub struct RewardEstimate {
    /// The amount of **decimal** coins that are going to get distributed to the node,
    /// i.e. the operator and all its delegators.
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub total_node_reward: Decimal,

    // note that operator reward includes the operating_cost,
    // i.e. say total_node_reward was `1nym` and operating_cost was `2nym`
    // in that case the operator reward would still be `1nym` as opposed to 0
    /// The share of the reward that is going to get distributed to the node operator.
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub operator: Decimal,

    /// The share of the reward that is going to get distributed among the node delegators.
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub delegates: Decimal,

    /// The operating cost of this node. Note: it's already included in the operator reward.
    #[cfg_attr(feature = "generate-ts", ts(type = "string"))]
    pub operating_cost: Decimal,
}

#[cw_serde]
#[derive(Copy, Default)]
pub struct RewardDistribution {
    /// The share of the reward that is going to get distributed to the node operator.
    pub operator: Decimal,

    /// The share of the reward that is going to get distributed among the node delegators.
    pub delegates: Decimal,
}

/// Response containing information about accrued rewards.
#[cw_serde]
#[derive(Default)]
pub struct PendingRewardResponse {
    /// The amount of tokens initially staked.
    pub amount_staked: Option<Coin>,

    /// The amount of tokens that could be claimed.
    pub amount_earned: Option<Coin>,

    /// The full pending rewards. Note that it's nearly identical to `amount_earned`,
    /// however, it contains few additional decimal points for more accurate reward calculation.
    pub amount_earned_detailed: Option<Decimal>,

    /// The associated mixnode is still fully bonded, meaning it is neither unbonded
    /// nor in the process of unbonding that would have finished at the epoch transition.
    #[deprecated(note = "this field will be removed. use .node_still_fully_bonded instead")]
    pub mixnode_still_fully_bonded: bool,

    pub node_still_fully_bonded: bool,
}

/// Response containing estimation of node rewards for the current epoch.
#[cw_serde]
pub struct EstimatedCurrentEpochRewardResponse {
    /// The amount of tokens initially staked.
    pub original_stake: Option<Coin>,

    /// The current stake value given all past rewarding and compounding since the original staking was performed.
    pub current_stake_value: Option<Coin>,

    /// The current stake value. Note that it's nearly identical to `current_stake_value`,
    /// however, it contains few additional decimal points for more accurate reward calculation.
    pub current_stake_value_detailed_amount: Option<Decimal>,

    /// The reward estimation for the current epoch, i.e. the amount of tokens that could be claimable
    /// after the epoch finishes and the state of the network does not change.
    pub estimation: Option<Coin>,

    /// The full reward estimation. Note that it's nearly identical to `estimation`,
    /// however, it contains few additional decimal points for more accurate reward calculation.
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

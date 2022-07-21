// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Coin, Decimal};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod helpers;
pub mod simulator;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct RewardEstimate {
    pub total_node_reward: Decimal,

    // note that operator reward includes the operating_cost,
    // i.e. say total_node_reward was `1nym` and operating_cost was `2nym`
    // in that case the operator reward would still be `1nym` as opposed to 0
    pub operator: Decimal,
    pub delegates: Decimal,
    pub operating_cost: Decimal,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct RewardDistribution {
    pub operator: Decimal,
    pub delegates: Decimal,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct PendingRewardResponse {
    pub amount_staked: Option<Coin>,
    pub amount_earned: Option<Coin>,
    pub amount_earned_detailed: Option<Decimal>,

    /// The associated mixnode is still fully bonded, meaning it is neither unbonded
    /// nor in the process of unbonding that would have finished at the epoch transition.
    pub mixnode_still_fully_bonded: bool,
}

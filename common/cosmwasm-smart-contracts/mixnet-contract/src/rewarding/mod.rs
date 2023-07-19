// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{MixId, RewardedSetNodeStatus};
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

#[cw_serde]
#[derive(Copy, Default)]
pub struct RewardDistribution {
    pub operator: Decimal,
    pub delegates: Decimal,
}

#[cw_serde]
#[derive(Default)]
pub struct PendingRewardResponse {
    pub amount_staked: Option<Coin>,
    pub amount_earned: Option<Coin>,
    pub amount_earned_detailed: Option<Decimal>,

    /// The associated mixnode is still fully bonded, meaning it is neither unbonded
    /// nor in the process of unbonding that would have finished at the epoch transition.
    pub mixnode_still_fully_bonded: bool,
}

#[cw_serde]
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

#[cw_serde]
pub struct PagedRewardedSetResponse {
    pub nodes: Vec<(MixId, RewardedSetNodeStatus)>,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<MixId>,
}

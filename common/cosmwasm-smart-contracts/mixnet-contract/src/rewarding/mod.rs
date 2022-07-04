// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod helpers;
pub mod simulator;

// since we're going to be storing a lot of those and json in all its wisdom keeps field names
// thus we use the rename to slightly save on space
/// HistoricalRewards represents historical rewards for a mixnode for a given period that is implicit
/// from the storage key.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct HistoricalRewards {
    /// Sum from the zeroth period until this period of rewards for the "unit delegation".
    // TODO: can we keep this as a Decimal with implicit Denom or should we rather "implement" a DecCoin?
    #[serde(rename = "crr")]
    pub cumulative_reward_ratio: Decimal,

    /// Number of outstanding delegations which ended the associated period and still might need
    /// to read this record.
    /// (+ one for the zeroth period, set on initialisation)
    #[serde(rename = "rc")]
    pub reference_count: u32,
}

impl HistoricalRewards {
    pub fn new(cumulative_reward_ratio: Decimal) -> Self {
        HistoricalRewards {
            cumulative_reward_ratio,
            reference_count: 1,
        }
    }

    pub fn increment_reference_count(&mut self) {
        self.reference_count += 1;
    }

    pub fn new_zeroth() -> Self {
        HistoricalRewards {
            cumulative_reward_ratio: Decimal::zero(),
            reference_count: 1,
        }
    }
}

// TODO: should this be put inside contract instead?
#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
pub struct RewardDistribution {
    pub operator: Decimal,
    pub delegates: Decimal,
}

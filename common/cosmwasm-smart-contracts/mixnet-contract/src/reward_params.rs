// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{error::MixnetContractError, Percent};
use cosmwasm_std::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub type Performance = Percent;

/// Parameters required by the mix-mining reward distribution that do not change during an interval.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, PartialOrd, Serialize, JsonSchema)]
pub struct IntervalRewardParams {
    /// Current value of the rewarding pool.
    /// It is expected to be constant throughout the interval.
    pub reward_pool: Decimal,

    /// Current value of the staking supply.
    /// It is expected to be constant throughout the interval.
    pub staking_supply: Decimal,

    // computed values
    /// Current value of the computed reward budget per epoch, per node.
    /// It is expected to be constant throughout the interval.
    pub epoch_reward_budget: Decimal,

    /// Current value of the stake saturation point.
    /// It is expected to be constant throughout the interval.
    pub stake_saturation_point: Decimal,

    // constants(-ish)
    // default: 30%
    /// Current value of the sybil resistance percent (`alpha`).
    /// It is not really expected to be changing very often.
    /// As a matter of fact, unless there's a very specific reason, it should remain constant.
    pub sybil_resistance: Percent,

    // default: 10
    /// Current active set work factor.
    /// It is not really expected to be changing very often.
    /// As a matter of fact, unless there's a very specific reason, it should remain constant.
    pub active_set_work_factor: Decimal,

    // default: 2%
    /// Current maximum interval pool emission.
    /// Assuming all nodes in the rewarded set are fully saturated and have 100% performance,
    /// this % of the reward pool would get distributed in rewards to all operators and its delegators.
    /// It is not really expected to be changing very often.
    /// As a matter of fact, unless there's a very specific reason, it should remain constant.
    pub interval_pool_emission: Percent,
}

impl IntervalRewardParams {
    pub fn to_inline_json(&self) -> String {
        // as per documentation on `to_string`:
        //      > Serialization can fail if `T`'s implementation of `Serialize` decides to
        //      > fail, or if `T` contains a map with non-string keys.
        // We have derived the `Serialize`, thus we're pretty confident it's valid and
        // the struct does not contain any maps, so the unwrap here is fine.
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, PartialOrd, Serialize, JsonSchema)]
pub struct RewardingParams {
    /// Parameters that should remain unchanged throughout an interval.
    pub interval: IntervalRewardParams,

    // while the active set size can change between epochs to accommodate for bandwidth demands,
    // the active set size should be unchanged between epochs and should only be adjusted between
    // intervals. However, it makes more sense to keep both of those values together as they're
    // very strongly related to each other.
    pub rewarded_set_size: u32,
    pub active_set_size: u32,
}

impl RewardingParams {
    pub fn active_node_work(&self) -> Decimal {
        self.interval.active_set_work_factor * self.standby_node_work()
    }

    pub fn standby_node_work(&self) -> Decimal {
        let f = self.interval.active_set_work_factor;
        let k = self.dec_rewarded_set_size();
        let one = Decimal::one();

        // nodes in reserve
        let k_r = self.dec_standby_set_size();

        one / (f * k - (f - one) * k_r)
    }

    pub(crate) fn dec_rewarded_set_size(&self) -> Decimal {
        // the unwrap here is fine as we're guaranteed an `u32` is going to fit in a Decimal
        // with 0 decimal places
        Decimal::from_atomics(self.rewarded_set_size, 0).unwrap()
    }

    fn dec_standby_set_size(&self) -> Decimal {
        // the unwrap here is fine as we're guaranteed an `u32` is going to fit in a Decimal
        // with 0 decimal places
        Decimal::from_atomics(self.rewarded_set_size - self.active_set_size, 0).unwrap()
    }

    pub fn apply_epochs_in_interval_change(&mut self, new_epochs_in_interval: u32) {
        self.interval.epoch_reward_budget = self.interval.reward_pool
            / Decimal::from_atomics(new_epochs_in_interval, 0).unwrap()
            * self.interval.interval_pool_emission;
    }

    pub fn try_change_active_set_size(
        &mut self,
        new_active_set_size: u32,
    ) -> Result<(), MixnetContractError> {
        if new_active_set_size == 0 {
            return Err(MixnetContractError::ZeroActiveSet);
        }

        if new_active_set_size > self.rewarded_set_size {
            return Err(MixnetContractError::InvalidActiveSetSize);
        }

        self.active_set_size = new_active_set_size;
        Ok(())
    }

    pub fn try_apply_updates(
        &mut self,
        updates: IntervalRewardingParamsUpdate,
        epochs_in_interval: u32,
    ) -> Result<(), MixnetContractError> {
        if !updates.contains_updates() {
            return Err(MixnetContractError::EmptyParamsChangeMsg);
        }

        let mut recompute_epoch_budget = false;
        let mut recompute_saturation_point = false;

        if let Some(reward_pool) = updates.reward_pool {
            recompute_epoch_budget = true;
            self.interval.reward_pool = reward_pool;
        }

        if let Some(staking_supply) = updates.staking_supply {
            recompute_saturation_point = true;
            self.interval.staking_supply = staking_supply;
        }

        if let Some(sybil_resistance_percent) = updates.sybil_resistance_percent {
            self.interval.sybil_resistance = sybil_resistance_percent;
        }

        if let Some(active_set_work_factor) = updates.active_set_work_factor {
            self.interval.active_set_work_factor = active_set_work_factor;
        }

        if let Some(interval_pool_emission) = updates.interval_pool_emission {
            recompute_epoch_budget = true;
            self.interval.interval_pool_emission = interval_pool_emission;
        }

        if let Some(rewarded_set_size) = updates.rewarded_set_size {
            if rewarded_set_size == 0 {
                return Err(MixnetContractError::ZeroRewardedSet);
            }
            if rewarded_set_size < self.active_set_size {
                return Err(MixnetContractError::InvalidRewardedSetSize);
            }

            recompute_saturation_point = true;
            self.rewarded_set_size = rewarded_set_size;
        }

        if recompute_epoch_budget {
            self.interval.epoch_reward_budget = self.interval.reward_pool
                / Decimal::from_atomics(epochs_in_interval, 0).unwrap()
                * self.interval.interval_pool_emission;
        }

        if recompute_saturation_point {
            self.interval.stake_saturation_point = self.interval.staking_supply
                / Decimal::from_atomics(self.rewarded_set_size, 0).unwrap();
        }

        Ok(())
    }
}

// TODO: possibly refactor this
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, PartialOrd, Serialize, JsonSchema)]
pub struct NodeRewardParams {
    pub performance: Percent,
    pub in_active_set: bool,
}

impl NodeRewardParams {
    pub fn new(performance: Percent, in_active_set: bool) -> Self {
        NodeRewardParams {
            performance,
            in_active_set,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, PartialOrd, Serialize, JsonSchema)]
pub struct IntervalRewardingParamsUpdate {
    pub reward_pool: Option<Decimal>,
    pub staking_supply: Option<Decimal>,

    pub sybil_resistance_percent: Option<Percent>,
    pub active_set_work_factor: Option<Decimal>,
    pub interval_pool_emission: Option<Percent>,
    pub rewarded_set_size: Option<u32>,
}

impl IntervalRewardingParamsUpdate {
    pub fn contains_updates(&self) -> bool {
        // essentially at least a single field has to be a `Some`
        self.reward_pool.is_some()
            || self.staking_supply.is_some()
            || self.sybil_resistance_percent.is_some()
            || self.active_set_work_factor.is_some()
            || self.interval_pool_emission.is_some()
            || self.rewarded_set_size.is_some()
    }

    pub fn to_inline_json(&self) -> String {
        // as per documentation on `to_string`:
        //      > Serialization can fail if `T`'s implementation of `Serialize` decides to
        //      > fail, or if `T` contains a map with non-string keys.
        // We have derived the `Serialize`, thus we're pretty confident it's valid and
        // the struct does not contain any maps, so the unwrap here is fine.
        serde_json::to_string(self).unwrap()
    }
}

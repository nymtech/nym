use crate::{error::MixnetContractError, Percent};
use cosmwasm_std::{Decimal, Uint128};
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
}
//
// #[derive(Debug, Clone, JsonSchema, PartialEq, Serialize, Deserialize, Copy)]
// pub struct NodeEpochRewards {
//     params: NodeRewardParams,
//     result: StoredNodeRewardResult,
//     epoch_id: u32,
// }

// impl NodeEpochRewards {
//     pub fn new(params: NodeRewardParams, result: StoredNodeRewardResult, epoch_id: u32) -> Self {
//         Self {
//             params,
//             result,
//             epoch_id,
//         }
//     }
//
//     pub fn epoch_id(&self) -> u32 {
//         self.epoch_id
//     }
//
//     pub fn sigma(&self) -> U128 {
//         self.result.sigma()
//     }
//
//     pub fn lambda(&self) -> U128 {
//         self.result.lambda()
//     }
//
//     pub fn params(&self) -> NodeRewardParams {
//         self.params
//     }
//
//     pub fn reward(&self) -> Uint128 {
//         self.result.reward()
//     }
//
//     pub fn operator_cost(&self, base_operator_cost: u64) -> U128 {
//         self.params.operator_cost(base_operator_cost)
//     }
//
//     pub fn node_profit(&self, base_operator_cost: u64) -> U128 {
//         let reward = U128::from_num(self.reward().u128());
//         // if operating cost is higher then the reward node profit is 0
//         reward.saturating_sub(self.operator_cost(base_operator_cost))
//     }
//
//     pub fn operator_reward(
//         &self,
//         profit_margin: U128,
//         base_operator_cost: u64,
//     ) -> Result<Uint128, MixnetContractError> {
//         let reward = self.node_profit(base_operator_cost);
//         let operator_base_reward = reward.min(self.operator_cost(base_operator_cost));
//         let div_by_zero_check = if let Some(value) = self.lambda().checked_div(self.sigma()) {
//             value
//         } else {
//             return Err(MixnetContractError::DivisionByZero);
//         };
//         let operator_reward = (profit_margin + (ONE - profit_margin) * div_by_zero_check) * reward;
//
//         let reward = (operator_reward + operator_base_reward).max(U128::from_num(0u128));
//
//         if let Some(int_reward) = reward.checked_cast() {
//             Ok(Uint128::new(int_reward))
//         } else {
//             Err(MixnetContractError::CastError)
//         }
//     }
//
//     pub fn delegation_reward(
//         &self,
//         delegation_amount: Uint128,
//         profit_margin: U128,
//         base_operator_cost: u64,
//         epoch_reward_params: EpochRewardParams,
//     ) -> Result<Uint128, MixnetContractError> {
//         // change all values into their fixed representations
//         let delegation_amount = U128::from_num(delegation_amount.u128());
//         let staking_supply = U128::from_num(epoch_reward_params.staking_supply());
//
//         let scaled_delegation_amount = delegation_amount / staking_supply;
//
//         let check_div_by_zero =
//             if let Some(value) = scaled_delegation_amount.checked_div(self.sigma()) {
//                 value
//             } else {
//                 return Err(MixnetContractError::DivisionByZero);
//             };
//
//         let delegator_reward =
//             (ONE - profit_margin) * check_div_by_zero * self.node_profit(base_operator_cost);
//
//         let reward = delegator_reward.max(U128::ZERO);
//         if let Some(int_reward) = reward.checked_cast() {
//             Ok(Uint128::new(int_reward))
//         } else {
//             Err(MixnetContractError::CastError)
//         }
//     }
// }
//
// #[derive(Debug, Clone, JsonSchema, PartialEq, Serialize, Deserialize, Copy)]
// pub struct EpochRewardParams {
//     epoch_reward_pool: Uint128,
//     rewarded_set_size: Uint128,
//     active_set_size: Uint128,
//     #[serde(alias = "circulating_supply")]
//     staking_supply: Uint128,
//     sybil_resistance_percent: u8,
//     active_set_work_factor: u8,
// }
//
// impl EpochRewardParams {
//     pub fn new(
//         epoch_reward_pool: u128,
//         rewarded_set_size: u128,
//         active_set_size: u128,
//         staking_supply: u128,
//         sybil_resistance_percent: u8,
//         active_set_work_factor: u8,
//     ) -> EpochRewardParams {
//         EpochRewardParams {
//             epoch_reward_pool: Uint128::new(epoch_reward_pool),
//             rewarded_set_size: Uint128::new(rewarded_set_size),
//             active_set_size: Uint128::new(active_set_size),
//             staking_supply: Uint128::new(staking_supply),
//             sybil_resistance_percent,
//             active_set_work_factor,
//         }
//     }
//
//     // technically it's identical to what would have been derived with a Default implementation,
//     // however, I prefer to be explicit about it, as a `Default::default` value makes no sense
//     // apart from the `ValidatorCacheInner` context, where this value is not going to be touched anyway
//     // (it's guarded behind an `initialised` flag)
//     pub fn new_empty() -> Self {
//         EpochRewardParams {
//             epoch_reward_pool: Uint128::new(0),
//             staking_supply: Uint128::new(0),
//             sybil_resistance_percent: 0,
//             rewarded_set_size: Uint128::new(0),
//             active_set_size: Uint128::new(0),
//             active_set_work_factor: 0,
//         }
//     }
//
//     pub fn rewarded_set_size(&self) -> u128 {
//         self.rewarded_set_size.u128()
//     }
//
//     pub fn active_set_size(&self) -> u128 {
//         self.active_set_size.u128()
//     }
//
//     pub fn staking_supply(&self) -> u128 {
//         self.staking_supply.u128()
//     }
//
//     pub fn epoch_reward_pool(&self) -> u128 {
//         self.epoch_reward_pool.u128()
//     }
//
//     pub fn sybil_resistance_percent(&self) -> u8 {
//         self.sybil_resistance_percent
//     }
//
//     pub fn active_set_work_factor(&self) -> u8 {
//         self.active_set_work_factor
//     }
// }
//
// #[derive(Debug, Clone, JsonSchema, PartialEq, Serialize, Deserialize, Copy)]
// pub struct NodeRewardParams {
//     reward_blockstamp: u64,
//     uptime: Uint128,
//     in_active_set: bool,
// }
//
// impl NodeRewardParams {
//     pub fn new(reward_blockstamp: u64, uptime: u128, in_active_set: bool) -> NodeRewardParams {
//         NodeRewardParams {
//             reward_blockstamp,
//             uptime: Uint128::new(uptime),
//             in_active_set,
//         }
//     }
//
//     pub fn operator_cost(&self, base_operator_cost: u64) -> U128 {
//         self.performance() * U128::from_num(base_operator_cost)
//     }
//
//     pub fn uptime(&self) -> Uint128 {
//         self.uptime
//     }
//
//     pub fn performance(&self) -> U128 {
//         U128::from_num(self.uptime.u128()) / U128::from_num(100)
//     }
//
//     pub fn set_reward_blockstamp(&mut self, blockstamp: u64) {
//         self.reward_blockstamp = blockstamp;
//     }
// }
//
// #[derive(Debug, Clone, JsonSchema, PartialEq, Serialize, Deserialize, Copy)]
// pub struct RewardParams {
//     pub epoch: EpochRewardParams,
//     pub node: NodeRewardParams,
// }
//
// impl RewardParams {
//     pub fn new(epoch: EpochRewardParams, node: NodeRewardParams) -> RewardParams {
//         RewardParams { epoch, node }
//     }
//
//     pub fn omega(&self) -> U128 {
//         // As per keybase://chat/nymtech#tokeneconomics/1179
//         let denom = self.active_set_work_factor() * U128::from_num(self.rewarded_set_size())
//             - (self.active_set_work_factor() - ONE) * U128::from_num(self.idle_nodes().u128());
//
//         if denom == 0 {
//             return U128::ZERO;
//         }
//
//         // Div by zero checked above
//         if self.in_active_set() {
//             // work_active = factor / (factor * self.network.k[month] - (factor - 1) * idle_nodes)
//             self.active_set_work_factor() / denom * self.rewarded_set_size()
//         } else {
//             // work_idle = 1 / (factor * self.network.k[month] - (factor - 1) * idle_nodes)
//             ONE / denom * self.rewarded_set_size()
//         }
//     }
//
//     pub fn idle_nodes(&self) -> Uint128 {
//         self.epoch.rewarded_set_size - self.epoch.active_set_size
//     }
//
//     pub fn active_set_work_factor(&self) -> U128 {
//         U128::from_num(self.epoch.active_set_work_factor)
//     }
//
//     pub fn in_active_set(&self) -> bool {
//         self.node.in_active_set
//     }
//
//     pub fn performance(&self) -> U128 {
//         self.node.performance()
//     }
//
//     pub fn set_reward_blockstamp(&mut self, blockstamp: u64) {
//         self.node.reward_blockstamp = blockstamp;
//     }
//
//     pub fn epoch_reward_pool(&self) -> u128 {
//         self.epoch.epoch_reward_pool.u128()
//     }
//
//     pub fn rewarded_set_size(&self) -> u128 {
//         self.epoch.rewarded_set_size.u128()
//     }
//
//     pub fn staking_supply(&self) -> u128 {
//         self.epoch.staking_supply.u128()
//     }
//
//     pub fn reward_blockstamp(&self) -> u64 {
//         self.node.reward_blockstamp
//     }
//
//     pub fn uptime(&self) -> Uint128 {
//         self.node.uptime()
//     }
//
//     pub fn one_over_k(&self) -> U128 {
//         ONE / U128::from_num(self.epoch.rewarded_set_size.u128())
//     }
//
//     pub fn alpha(&self) -> U128 {
//         U128::from_num(self.epoch.sybil_resistance_percent) / U128::from_num(100)
//     }
// }

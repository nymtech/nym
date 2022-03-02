use crate::{mixnode::StoredNodeRewardResult, ONE, U128};
use cosmwasm_std::Uint128;
use network_defaults::DEFAULT_OPERATOR_INTERVAL_COST;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, JsonSchema, PartialEq, Serialize, Deserialize, Copy)]
pub struct NodeEpochRewards {
    params: RewardParams,
    result: StoredNodeRewardResult,
}

impl NodeEpochRewards {
    pub fn new(params: RewardParams, result: StoredNodeRewardResult) -> Self {
        Self { params, result }
    }
}

#[derive(Debug, Clone, JsonSchema, PartialEq, Serialize, Deserialize, Copy)]
pub struct IntervalRewardParams {
    period_reward_pool: Uint128,
    rewarded_set_size: Uint128,
    active_set_size: Uint128,
    circulating_supply: Uint128,
    sybil_resistance_percent: u8,
    active_set_work_factor: u8,
}

impl IntervalRewardParams {
    pub fn new(
        period_reward_pool: u128,
        rewarded_set_size: u128,
        active_set_size: u128,
        circulating_supply: u128,
        sybil_resistance_percent: u8,
        active_set_work_factor: u8,
    ) -> IntervalRewardParams {
        IntervalRewardParams {
            period_reward_pool: Uint128::new(period_reward_pool),
            rewarded_set_size: Uint128::new(rewarded_set_size),
            active_set_size: Uint128::new(active_set_size),
            circulating_supply: Uint128::new(circulating_supply),
            sybil_resistance_percent,
            active_set_work_factor,
        }
    }

    // technically it's identical to what would have been derived with a Default implementation,
    // however, I prefer to be explicit about it, as a `Default::default` value makes no sense
    // apart from the `ValidatorCacheInner` context, where this value is not going to be touched anyway
    // (it's guarded behind an `initialised` flag)
    pub fn new_empty() -> Self {
        IntervalRewardParams {
            period_reward_pool: Uint128::new(0),
            circulating_supply: Uint128::new(0),
            sybil_resistance_percent: 0,
            rewarded_set_size: Uint128::new(0),
            active_set_size: Uint128::new(0),
            active_set_work_factor: 0,
        }
    }

    pub fn rewarded_set_size(&self) -> u128 {
        self.rewarded_set_size.u128()
    }

    pub fn active_set_size(&self) -> u128 {
        self.active_set_size.u128()
    }

    pub fn circulating_supply(&self) -> u128 {
        self.circulating_supply.u128()
    }

    pub fn period_reward_pool(&self) -> u128 {
        self.period_reward_pool.u128()
    }
}

#[derive(Debug, Clone, JsonSchema, PartialEq, Serialize, Deserialize, Copy)]
pub struct NodeRewardParams {
    reward_blockstamp: u64,
    uptime: Uint128,
    in_active_set: bool,
}

impl NodeRewardParams {
    #[allow(clippy::too_many_arguments)]
    pub fn new(reward_blockstamp: u64, uptime: u128, in_active_set: bool) -> NodeRewardParams {
        NodeRewardParams {
            reward_blockstamp,
            uptime: Uint128::new(uptime),
            in_active_set,
        }
    }
}

#[derive(Debug, Clone, JsonSchema, PartialEq, Serialize, Deserialize, Copy)]
pub struct RewardParams {
    interval: IntervalRewardParams,
    node: NodeRewardParams,
}

impl RewardParams {
    pub fn new(interval: IntervalRewardParams, node: NodeRewardParams) -> RewardParams {
        RewardParams { interval, node }
    }

    pub fn omega(&self) -> U128 {
        // As per keybase://chat/nymtech#tokeneconomics/1179
        let denom = self.active_set_work_factor() * U128::from_num(self.rewarded_set_size())
            - (self.active_set_work_factor() - ONE) * U128::from_num(self.idle_nodes().u128());

        if self.in_active_set() {
            // work_active = factor / (factor * self.network.k[month] - (factor - 1) * idle_nodes)
            self.active_set_work_factor() / denom * self.rewarded_set_size()
        } else {
            // work_idle = 1 / (factor * self.network.k[month] - (factor - 1) * idle_nodes)
            ONE / denom * self.rewarded_set_size()
        }
    }

    pub fn idle_nodes(&self) -> Uint128 {
        self.interval.rewarded_set_size - self.interval.active_set_size
    }

    pub fn active_set_work_factor(&self) -> U128 {
        U128::from_num(self.interval.active_set_work_factor)
    }

    pub fn in_active_set(&self) -> bool {
        self.node.in_active_set
    }

    pub fn performance(&self) -> U128 {
        U128::from_num(self.node.uptime.u128()) / U128::from_num(100)
    }

    pub fn operator_cost(&self) -> U128 {
        U128::from_num(self.node.uptime.u128() / 100u128 * DEFAULT_OPERATOR_INTERVAL_COST as u128)
    }

    pub fn set_reward_blockstamp(&mut self, blockstamp: u64) {
        self.node.reward_blockstamp = blockstamp;
    }

    pub fn period_reward_pool(&self) -> u128 {
        self.interval.period_reward_pool.u128()
    }

    pub fn rewarded_set_size(&self) -> u128 {
        self.interval.rewarded_set_size.u128()
    }

    pub fn circulating_supply(&self) -> u128 {
        self.interval.circulating_supply.u128()
    }

    pub fn reward_blockstamp(&self) -> u64 {
        self.node.reward_blockstamp
    }

    pub fn uptime(&self) -> u128 {
        self.node.uptime.u128()
    }

    pub fn one_over_k(&self) -> U128 {
        ONE / U128::from_num(self.interval.rewarded_set_size.u128())
    }

    pub fn alpha(&self) -> U128 {
        U128::from_num(self.interval.sybil_resistance_percent) / U128::from_num(100)
    }
}

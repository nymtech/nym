use crate::{error::MixnetContractError, mixnode::StoredNodeRewardResult, ONE, U128};
use az::CheckedCast;
use cosmwasm_std::Uint128;
use network_defaults::DEFAULT_OPERATOR_INTERVAL_COST;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, JsonSchema, PartialEq, Serialize, Deserialize, Copy)]
pub struct NodeEpochRewards {
    params: NodeRewardParams,
    result: StoredNodeRewardResult,
    epoch_id: u32,
}

impl NodeEpochRewards {
    pub fn new(params: NodeRewardParams, result: StoredNodeRewardResult, epoch_id: u32) -> Self {
        Self {
            params,
            result,
            epoch_id,
        }
    }

    pub fn epoch_id(&self) -> u32 {
        self.epoch_id
    }

    pub fn sigma(&self) -> Uint128 {
        self.result.sigma()
    }

    pub fn lambda(&self) -> Uint128 {
        self.result.lambda()
    }

    pub fn params(&self) -> NodeRewardParams {
        self.params
    }

    pub fn reward(&self) -> Uint128 {
        self.result.reward()
    }

    pub fn operator_cost(&self) -> U128 {
        U128::from_num(self.params.uptime.u128() / 100u128 * DEFAULT_OPERATOR_INTERVAL_COST as u128)
    }

    pub fn node_profit(&self) -> U128 {
        let reward = U128::from_num(self.reward().u128());
        if reward < self.operator_cost() {
            U128::from_num(0u128)
        } else {
            reward - self.operator_cost()
        }
    }

    pub fn operator_reward(&self, profit_margin: U128) -> Result<Uint128, MixnetContractError> {
        let reward = self.node_profit();
        let operator_base_reward = reward.min(self.operator_cost());
        let operator_reward = (profit_margin
            + (ONE - profit_margin) * U128::from_num(self.lambda().u128())
                / U128::from_num(self.sigma().u128()))
            * reward;

        let reward = (operator_reward + operator_base_reward).max(U128::from_num(0u128));

        if let Some(int_reward) = reward.checked_cast() {
            Ok(Uint128::new(int_reward))
        } else {
            Err(MixnetContractError::CastError)
        }
    }

    pub fn delegation_reward(
        &self,
        delegation_amount: Uint128,
        profit_margin: U128,
        epoch_reward_params: EpochRewardParams,
    ) -> Result<Uint128, MixnetContractError> {
        // change all values into their fixed representations
        let delegation_amount = U128::from_num(delegation_amount.u128());
        let circulating_supply = U128::from_num(epoch_reward_params.circulating_supply());

        let scaled_delegation_amount = delegation_amount / circulating_supply;
        let delegator_reward = (ONE - profit_margin) * scaled_delegation_amount
            / U128::from_num(self.sigma().u128())
            * self.node_profit();

        let reward = delegator_reward.max(U128::ZERO);
        if let Some(int_reward) = reward.checked_cast() {
            Ok(Uint128::new(int_reward))
        } else {
            Err(MixnetContractError::CastError)
        }
    }
}

#[derive(Debug, Clone, JsonSchema, PartialEq, Serialize, Deserialize, Copy)]
#[cfg_attr(feature = "ts-rs", derive(ts_rs::TS))]
pub struct EpochRewardParams {
    #[cfg_attr(feature = "ts-rs", ts(type = "string"))]
    epoch_reward_pool: Uint128,

    #[cfg_attr(feature = "ts-rs", ts(type = "string"))]
    rewarded_set_size: Uint128,

    #[cfg_attr(feature = "ts-rs", ts(type = "string"))]
    active_set_size: Uint128,

    #[cfg_attr(feature = "ts-rs", ts(type = "string"))]
    circulating_supply: Uint128,
    sybil_resistance_percent: u8,
    active_set_work_factor: u8,
}

impl EpochRewardParams {
    pub fn new(
        epoch_reward_pool: u128,
        rewarded_set_size: u128,
        active_set_size: u128,
        circulating_supply: u128,
        sybil_resistance_percent: u8,
        active_set_work_factor: u8,
    ) -> EpochRewardParams {
        EpochRewardParams {
            epoch_reward_pool: Uint128::new(epoch_reward_pool),
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
        EpochRewardParams {
            epoch_reward_pool: Uint128::new(0),
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

    pub fn epoch_reward_pool(&self) -> u128 {
        self.epoch_reward_pool.u128()
    }
}

#[derive(Debug, Clone, JsonSchema, PartialEq, Serialize, Deserialize, Copy)]
#[cfg_attr(feature = "ts-rs", derive(ts_rs::TS))]
pub struct NodeRewardParams {
    reward_blockstamp: u64,
    #[cfg_attr(feature = "ts-rs", ts(type = "string"))]
    uptime: Uint128,
    in_active_set: bool,
}

impl NodeRewardParams {
    pub fn new(reward_blockstamp: u64, uptime: u128, in_active_set: bool) -> NodeRewardParams {
        NodeRewardParams {
            reward_blockstamp,
            uptime: Uint128::new(uptime),
            in_active_set,
        }
    }

    pub fn operator_cost(&self) -> U128 {
        U128::from_num(self.uptime.u128() / 100u128 * DEFAULT_OPERATOR_INTERVAL_COST as u128)
    }

    pub fn uptime(&self) -> u128 {
        self.uptime.u128()
    }

    pub fn set_reward_blockstamp(&mut self, blockstamp: u64) {
        self.reward_blockstamp = blockstamp;
    }
}

#[derive(Debug, Clone, JsonSchema, PartialEq, Serialize, Deserialize, Copy)]
#[cfg_attr(feature = "ts-rs", derive(ts_rs::TS))]
pub struct RewardParams {
    pub epoch: EpochRewardParams,
    pub node: NodeRewardParams,
}

impl RewardParams {
    pub fn new(epoch: EpochRewardParams, node: NodeRewardParams) -> RewardParams {
        RewardParams { epoch, node }
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
        self.epoch.rewarded_set_size - self.epoch.active_set_size
    }

    pub fn active_set_work_factor(&self) -> U128 {
        U128::from_num(self.epoch.active_set_work_factor)
    }

    pub fn in_active_set(&self) -> bool {
        self.node.in_active_set
    }

    pub fn performance(&self) -> U128 {
        U128::from_num(self.node.uptime.u128()) / U128::from_num(100)
    }

    pub fn set_reward_blockstamp(&mut self, blockstamp: u64) {
        self.node.reward_blockstamp = blockstamp;
    }

    pub fn epoch_reward_pool(&self) -> u128 {
        self.epoch.epoch_reward_pool.u128()
    }

    pub fn rewarded_set_size(&self) -> u128 {
        self.epoch.rewarded_set_size.u128()
    }

    pub fn circulating_supply(&self) -> u128 {
        self.epoch.circulating_supply.u128()
    }

    pub fn reward_blockstamp(&self) -> u64 {
        self.node.reward_blockstamp
    }

    pub fn uptime(&self) -> u128 {
        self.node.uptime.u128()
    }

    pub fn one_over_k(&self) -> U128 {
        ONE / U128::from_num(self.epoch.rewarded_set_size.u128())
    }

    pub fn alpha(&self) -> U128 {
        U128::from_num(self.epoch.sybil_resistance_percent) / U128::from_num(100)
    }
}

// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use config::defaults::TOTAL_SUPPLY;
use cosmwasm_std::{Env, Order, StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map, U32Key};
use mixnet_contract::{IdentityKey, NodeStatus, RewardingStatus};
use std::collections::HashMap;

pub(crate) const REWARD_POOL: Item<Uint128> = Item::new("pool");
pub(crate) const REWARDING_STATUS: Map<(U32Key, IdentityKey), RewardingStatus> = Map::new("rm");
pub(crate) const REWARDED_SET_HEIGHTS: Map<u64, ()> = Map::new("rsh");
pub(crate) const REWARDED_SET: Map<(u64, IdentityKey), NodeStatus> = Map::new("rs");

// approximately 1 day (assuming 5s per block)
pub(crate) const MINIMUM_BLOCK_AGE_FOR_REWARDING: u64 = 17280;

// approximately 30min (assuming 5s per block)
pub(crate) const MAX_REWARDING_DURATION_IN_BLOCKS: u64 = 360;

pub fn update_rewarded_set(
    rewarded_set: HashMap<IdentityKey, NodeStatus>,
    storage: &mut dyn Storage,
    env: Env,
) -> Result<(), ContractError> {
    let block_height = env.block.height;
    for (key, value) in rewarded_set {
        REWARDED_SET.save(storage, (block_height, key), &value)?;
    }
    REWARDED_SET_HEIGHTS.save(storage, block_height, &())?;
    Ok(())
}

pub fn latest_rewarded_set_height(storage: &dyn Storage) -> Result<u64, ContractError> {
    if let Some(Ok(height)) = REWARDED_SET_HEIGHTS
        .keys_de(storage, None, None, Order::Descending)
        .next()
    {
        Ok(height)
    } else {
        Err(ContractError::RewardSetHeightMapEmpty)
    }
}

pub fn rewarded_set_at_height(
    height: u64,
    storage: &dyn Storage,
) -> Result<HashMap<IdentityKey, NodeStatus>, ContractError> {
    let rewarded_set: StdResult<Vec<_>> = REWARDED_SET
        .prefix_de(height)
        .range(storage, None, None, Order::Ascending)
        .collect();
    Ok(rewarded_set?.into_iter().collect())
}

pub fn rewarded_set(
    storage: &dyn Storage,
) -> Result<HashMap<IdentityKey, NodeStatus>, ContractError> {
    let latest_height = latest_rewarded_set_height(storage)?;
    rewarded_set_at_height(latest_height, storage)
}

#[allow(dead_code)]
pub fn incr_reward_pool(
    amount: Uint128,
    storage: &mut dyn Storage,
) -> Result<Uint128, ContractError> {
    REWARD_POOL.update(storage, |mut current_pool| {
        current_pool += amount;
        Ok(current_pool)
    })
}

pub fn decr_reward_pool(
    storage: &mut dyn Storage,
    amount: Uint128,
) -> Result<Uint128, ContractError> {
    REWARD_POOL.update(storage, |current_pool| {
        let stake = current_pool
            .checked_sub(amount)
            .map_err(|_| ContractError::OutOfFunds {
                to_remove: amount.u128(),
                reward_pool: current_pool.u128(),
            })?;

        Ok(stake)
    })
}

pub fn circulating_supply(storage: &dyn Storage) -> StdResult<Uint128> {
    let reward_pool = REWARD_POOL.load(storage)?;
    Ok(Uint128::new(TOTAL_SUPPLY) - reward_pool)
}

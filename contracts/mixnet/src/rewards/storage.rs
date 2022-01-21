// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use config::defaults::TOTAL_SUPPLY;
use cosmwasm_std::{StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map};
use mixnet_contract_common::{IdentityKey, RewardingStatus};

pub(crate) const REWARD_POOL: Item<Uint128> = Item::new("pool");
// TODO: Do we need a migration for this?
pub(crate) const REWARDING_STATUS: Map<(u32, IdentityKey), RewardingStatus> = Map::new("rm");

// approximately 1 week (assuming 5s per block)
// i.e. approximately quarter of the interval (there are 3600 * 60 * 7 = 604800 seconds in a week, i.e. ~604800 / 5 = 120960 blocks)
pub(crate) const MINIMUM_BLOCK_AGE_FOR_REWARDING: u64 = 120960;

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

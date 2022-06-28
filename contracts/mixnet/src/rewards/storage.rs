// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use crate::mixnet_contract_settings::storage as settings_storage;
use config::defaults::TOTAL_SUPPLY;
use cosmwasm_std::{StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map};
use mixnet_contract_common::{reward_params::EpochRewardParams, IdentityKey, RewardingStatus};

type BlockHeight = u64;
type Address = String;

pub(crate) const REWARD_POOL: Item<'_, Uint128> = Item::new("pool");
// TODO: Do we need a migration for this?
pub(crate) const REWARDING_STATUS: Map<'_, (u32, IdentityKey), RewardingStatus> = Map::new("rm");

// This has to be a byte vector due to proxy delegastions and rewarding
pub(crate) const DELEGATOR_REWARD_CLAIMED_HEIGHT: Map<'_, (Vec<u8>, IdentityKey), BlockHeight> =
    Map::new("drc");
pub(crate) const OPERATOR_REWARD_CLAIMED_HEIGHT: Map<'_, (Address, IdentityKey), BlockHeight> =
    Map::new("orc");

type EpochId = u32;

pub(crate) const EPOCH_REWARD_PARAMS: Map<'_, EpochId, EpochRewardParams> = Map::new("epr");

pub fn epoch_reward_params_for_id(
    storage: &dyn Storage,
    id: EpochId,
) -> StdResult<EpochRewardParams> {
    EPOCH_REWARD_PARAMS.load(storage, id)
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

pub fn reward_accounting(storage: &mut dyn Storage, amount: Uint128) -> Result<(), ContractError> {
    decr_reward_pool(storage, amount)?;
    incr_staking_supply(storage, amount)?;
    Ok(())
}

fn incr_staking_supply(storage: &mut dyn Storage, amount: Uint128) -> Result<(), ContractError> {
    let mut contract_state =
        crate::mixnet_contract_settings::storage::CONTRACT_STATE.load(storage)?;
    contract_state.params.staking_supply += amount;
    crate::mixnet_contract_settings::storage::CONTRACT_STATE.save(storage, &contract_state)?;
    Ok(())
}

fn decr_reward_pool(storage: &mut dyn Storage, amount: Uint128) -> Result<Uint128, ContractError> {
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
    Ok(Uint128::new(TOTAL_SUPPLY).saturating_sub(reward_pool))
}

pub fn staking_supply(storage: &dyn Storage) -> StdResult<Uint128> {
    let state = settings_storage::CONTRACT_STATE.load(storage)?;
    Ok(state.params.staking_supply)
}

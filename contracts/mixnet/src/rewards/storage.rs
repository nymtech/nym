// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{
    MIXNODES_REWARDING_PK_NAMESPACE, PENDING_REWARD_POOL_KEY, REWARDING_PARAMS_KEY,
};
use crate::rewards::models::RewardPoolChange;
use cosmwasm_std::{Decimal, StdResult, Storage};
use cw_storage_plus::{Item, Map};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::mixnode::MixNodeRewarding;
use mixnet_contract_common::reward_params::RewardingParams;
use mixnet_contract_common::{InitialRewardingParams, NodeId};

// current parameters used for rewarding purposes
pub(crate) const REWARDING_PARAMS: Item<'_, RewardingParams> = Item::new(REWARDING_PARAMS_KEY);
pub(crate) const PENDING_REWARD_POOL_CHANGE: Item<'_, RewardPoolChange> =
    Item::new(PENDING_REWARD_POOL_KEY);

pub const MIXNODE_REWARDING: Map<NodeId, MixNodeRewarding> =
    Map::new(MIXNODES_REWARDING_PK_NAMESPACE);

pub fn reward_accounting(
    storage: &mut dyn Storage,
    amount: Decimal,
) -> Result<(), MixnetContractError> {
    let mut pending_changes = PENDING_REWARD_POOL_CHANGE.load(storage)?;
    pending_changes.removed += amount;

    Ok(PENDING_REWARD_POOL_CHANGE.save(storage, &pending_changes)?)
}

//
// pub fn circulating_supply(storage: &dyn Storage) -> StdResult<Uint128> {
//     let reward_pool = REWARD_POOL.load(storage)?;
//     Ok(Uint128::new(TOTAL_SUPPLY).saturating_sub(reward_pool))
// }
//
// pub fn staking_supply(storage: &dyn Storage) -> StdResult<Uint128> {
//     let state = settings_storage::CONTRACT_STATE.load(storage)?;
//     Ok(state.params.staking_supply)
// }

pub(crate) fn initialise_storage(
    storage: &mut dyn Storage,
    reward_params: RewardingParams,
) -> StdResult<()> {
    REWARDING_PARAMS.save(storage, &reward_params)?;
    PENDING_REWARD_POOL_CHANGE.save(storage, &RewardPoolChange::default())
}

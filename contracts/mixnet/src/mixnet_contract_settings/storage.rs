// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::CONTRACT_STATE_KEY;
use cosmwasm_std::{Addr, Storage};
use cosmwasm_std::{Coin, StdResult};
use cw_storage_plus::Item;
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::ContractState;

pub(crate) const CONTRACT_STATE: Item<'_, ContractState> = Item::new(CONTRACT_STATE_KEY);

pub fn rewarding_validator_address(storage: &dyn Storage) -> Result<Addr, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.rewarding_validator_address)?)
}

pub(crate) fn minimum_mixnode_pledge(storage: &dyn Storage) -> Result<Coin, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.params.minimum_mixnode_pledge)?)
}

pub(crate) fn minimum_gateway_pledge(storage: &dyn Storage) -> Result<Coin, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.params.minimum_gateway_pledge)?)
}

#[allow(unused)]
pub(crate) fn minimum_delegation_stake(
    storage: &dyn Storage,
) -> Result<Option<Coin>, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.params.minimum_mixnode_delegation)?)
}

pub(crate) fn rewarding_denom(storage: &dyn Storage) -> Result<String, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.rewarding_denom)?)
}

pub(crate) fn initialise_storage(
    storage: &mut dyn Storage,
    initial_state: ContractState,
) -> StdResult<()> {
    CONTRACT_STATE.save(storage, &initial_state)
}

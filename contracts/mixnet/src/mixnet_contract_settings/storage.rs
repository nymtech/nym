// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{ADMIN_STORAGE_KEY, CONTRACT_STATE_KEY};
use cosmwasm_std::{Addr, DepsMut, Storage};
use cosmwasm_std::{Coin, StdResult};
use cw_controllers::Admin;
use cw_storage_plus::Item;
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::{
    ContractState, ContractStateParams, OperatingCostRange, ProfitMarginRange,
};

pub(crate) const CONTRACT_STATE: Item<'_, ContractState> = Item::new(CONTRACT_STATE_KEY);
pub(crate) const ADMIN: Admin = Admin::new(ADMIN_STORAGE_KEY);

pub fn rewarding_validator_address(storage: &dyn Storage) -> Result<Addr, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.rewarding_validator_address)?)
}

pub(crate) fn minimum_node_pledge(storage: &dyn Storage) -> Result<Coin, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.params.minimum_pledge)?)
}

pub(crate) fn profit_margin_range(
    storage: &dyn Storage,
) -> Result<ProfitMarginRange, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.params.profit_margin)?)
}

pub(crate) fn interval_oprating_cost_range(
    storage: &dyn Storage,
) -> Result<OperatingCostRange, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.params.interval_operating_cost)?)
}

#[allow(unused)]
pub(crate) fn minimum_delegation_stake(
    storage: &dyn Storage,
) -> Result<Option<Coin>, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.params.minimum_delegation)?)
}

pub(crate) fn rewarding_denom(storage: &dyn Storage) -> Result<String, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.rewarding_denom)?)
}

pub(crate) fn vesting_contract_address(storage: &dyn Storage) -> Result<Addr, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.vesting_contract_address)?)
}

pub(crate) fn state_params(
    storage: &dyn Storage,
) -> Result<ContractStateParams, MixnetContractError> {
    Ok(CONTRACT_STATE.load(storage).map(|state| state.params)?)
}

pub(crate) fn initialise_storage(
    deps: DepsMut<'_>,
    initial_state: ContractState,
    initial_admin: Addr,
) -> StdResult<()> {
    CONTRACT_STATE.save(deps.storage, &initial_state)?;
    ADMIN.set(deps, Some(initial_admin))
}

// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnet_contract_settings::models::ContractState;
use cosmwasm_std::Storage;
use cosmwasm_std::{Coin, StdResult};
use cw_storage_plus::Item;
use mixnet_contract_common::error::MixnetContractError;

pub(crate) const CONTRACT_STATE: Item<'_, ContractState> = Item::new("config");

pub fn rewarding_validator_address(storage: &dyn Storage) -> Result<String, MixnetContractError> {
    Ok(CONTRACT_STATE
        .load(storage)
        .map(|state| state.rewarding_validator_address.to_string())?)
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

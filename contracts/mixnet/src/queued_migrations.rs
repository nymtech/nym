// Copyright 2022-2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use cosmwasm_std::{Addr, DepsMut};
use mixnet_contract_common::error::MixnetContractError;

pub fn introduce_directory_contract(
    deps: DepsMut,
    directory_contract_address: Addr,
) -> Result<(), MixnetContractError> {
    // the load using the current structure is fine as we've added optional field
    let mut state = mixnet_params_storage::CONTRACT_STATE.load(deps.storage)?;
    state.directory_contract_address = Some(directory_contract_address);
    mixnet_params_storage::CONTRACT_STATE.save(deps.storage, &state)?;

    Ok(())
}

pub fn introduce_geolocation_contract(
    deps: DepsMut,
    geolocation_contract_address: Addr,
) -> Result<(), MixnetContractError> {
    // the load using the current structure is fine as we've added optional field
    let mut state = mixnet_params_storage::CONTRACT_STATE.load(deps.storage)?;
    state.geolocation_contract_address = Some(geolocation_contract_address);
    mixnet_params_storage::CONTRACT_STATE.save(deps.storage, &state)?;

    Ok(())
}

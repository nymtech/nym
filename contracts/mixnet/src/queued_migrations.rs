// Copyright 2022-2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use cosmwasm_std::{Addr, DepsMut};
use mixnet_contract_common::error::MixnetContractError;

pub fn introduce_performance_contract(
    deps: DepsMut,
    performance_contract_address: Addr,
) -> Result<(), MixnetContractError> {
    // the load using the current structure is fine as we've added optional field
    let mut state = mixnet_params_storage::CONTRACT_STATE.load(deps.storage)?;
    state.performance_contract_address = Some(performance_contract_address);
    mixnet_params_storage::CONTRACT_STATE.save(deps.storage, &state)?;

    Ok(())
}

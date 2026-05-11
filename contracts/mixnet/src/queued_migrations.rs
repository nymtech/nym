// Copyright 2022-2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::CONTRACT_STATE_KEY;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use cosmwasm_std::{Addr, DepsMut};
use cw_storage_plus::Item;
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::{ContractState, ContractStateParams};

pub fn introduce_node_families_contract(
    deps: DepsMut,
    node_families_contract_address: Addr,
) -> Result<(), MixnetContractError> {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct OldContractState {
        owner: Option<Addr>,
        rewarding_validator_address: Addr,
        vesting_contract_address: Addr,
        rewarding_denom: String,
        params: ContractStateParams,
    }

    const OLD_CONTRACT_STATE: Item<OldContractState> = Item::new(CONTRACT_STATE_KEY);
    let old = OLD_CONTRACT_STATE.load(deps.storage)?;

    #[allow(deprecated)]
    let updated = ContractState {
        owner: old.owner,
        rewarding_validator_address: old.rewarding_validator_address,
        vesting_contract_address: old.vesting_contract_address,
        rewarding_denom: old.rewarding_denom,
        params: old.params,
        node_families_contract_address,
    };
    mixnet_params_storage::CONTRACT_STATE.save(deps.storage, &updated)?;

    Ok(())
}

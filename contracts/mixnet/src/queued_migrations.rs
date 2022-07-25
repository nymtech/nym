// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, DepsMut, Env, Response};
use cw_storage_plus::Item;
use mixnet_contract_common::{ContractStateParams, MigrateMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::ContractError;
use crate::mixnet_contract_settings::models::ContractState;
use crate::mixnet_contract_settings::storage::CONTRACT_STATE;

pub fn migrate_config_from_env(
    deps: DepsMut<'_>,
    _env: Env,
    msg: MigrateMsg,
) -> Result<Response, ContractError> {
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
    pub struct OldContractState {
        pub owner: Addr,
        pub mix_denom: String,
        pub rewarding_validator_address: Addr,
        pub params: ContractStateParams,
    }
    const OLD_CONTRACT_STATE: Item<'_, OldContractState> = Item::new("config");

    let old_state = OLD_CONTRACT_STATE.load(deps.storage)?;
    let new_state = ContractState {
        owner: old_state.owner,
        mix_denom: msg.mixnet_denom,
        rewarding_validator_address: old_state.rewarding_validator_address,
        params: old_state.params,
    };

    CONTRACT_STATE.save(deps.storage, &new_state)?;

    Ok(Default::default())
}

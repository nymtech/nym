// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{entry_point, Addr, Coin, DepsMut, Empty, Env, Response};
use cw3_flex_multisig::{state::CONFIG, ContractError};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const OWNER: &str = "admin0001";
pub const VOTER1: &str = "voter1";
pub const VOTER2: &str = "voter2";
pub const VOTER3: &str = "voter3";
pub const MULTISIG_CONTRACT: &str = "multisig contract address";
pub const POOL_CONTRACT: &str = "mix pool contract address";
pub const RANDOM_ADDRESS: &str = "random address";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {
    pub coconut_bandwidth_address: String,
}

#[entry_point]
pub fn migrate(deps: DepsMut<'_>, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    let mut cfg = CONFIG.load(deps.storage)?;
    cfg.coconut_bandwidth_addr = deps.api.addr_validate(&msg.coconut_bandwidth_address)?;
    CONFIG.save(deps.storage, &cfg)?;
    Ok(Default::default())
}

pub fn mock_app(init_funds: &[Coin]) -> App {
    AppBuilder::new().build(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &Addr::unchecked(OWNER), init_funds.to_vec())
            .unwrap();
    })
}

pub fn contract_bandwidth() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        coconut_bandwidth::contract::execute,
        coconut_bandwidth::contract::instantiate,
        coconut_bandwidth::contract::query,
    );
    Box::new(contract)
}

pub fn contract_multisig() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw3_flex_multisig::contract::execute,
        cw3_flex_multisig::contract::instantiate,
        cw3_flex_multisig::contract::query,
    )
    .with_migrate(migrate);
    Box::new(contract)
}

pub fn contract_group() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw4_group::contract::execute,
        cw4_group::contract::instantiate,
        cw4_group::contract::query,
    );
    Box::new(contract)
}

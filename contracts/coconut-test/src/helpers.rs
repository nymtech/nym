// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{entry_point, Addr, Coin, DepsMut, Empty, Env, Response};
use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper};
use nym_multisig_contract_common::error::ContractError;
use nym_multisig_contract_common::state::CONFIG;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const OWNER: &str = "admin0001";
pub const MEMBER1: &str = "member1";
pub const MULTISIG_CONTRACT: &str = "multisig contract address";
pub const POOL_CONTRACT: &str = "mix pool contract address";
pub const RANDOM_ADDRESS: &str = "random address";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {
    pub coconut_bandwidth_address: String,
    pub coconut_dkg_address: String,
}

pub fn mock_app(init_funds: &[Coin]) -> App {
    AppBuilder::new().build(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &Addr::unchecked(OWNER), init_funds.to_vec())
            .unwrap();
        router
            .bank
            .init_balance(storage, &Addr::unchecked(MEMBER1), init_funds.to_vec())
            .unwrap();
    })
}
pub fn contract_dkg() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        nym_coconut_dkg::contract::execute,
        nym_coconut_dkg::contract::instantiate,
        nym_coconut_dkg::contract::query,
    );
    Box::new(contract)
}

pub fn contract_bandwidth() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        nym_coconut_bandwidth::contract::execute,
        nym_coconut_bandwidth::contract::instantiate,
        nym_coconut_bandwidth::contract::query,
    );
    Box::new(contract)
}

pub fn contract_multisig() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw3_flex_multisig::contract::execute,
        cw3_flex_multisig::contract::instantiate,
        cw3_flex_multisig::contract::query,
    )
    .with_migrate(cw3_flex_multisig::contract::migrate);
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

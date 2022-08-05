// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub const MULTISIG_CONTRACT: &str = "multisig contract address";
pub const POOL_CONTRACT: &str = "mix pool contract address";

use crate::contract::instantiate;
use coconut_bandwidth_contract_common::msg::InstantiateMsg;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier};
use cosmwasm_std::{Empty, MemoryStorage, OwnedDeps};

use super::fixtures::TEST_MIX_DENOM;

pub fn init_contract() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
    let mut deps = mock_dependencies();
    let msg = InstantiateMsg {
        multisig_addr: String::from(MULTISIG_CONTRACT),
        pool_addr: String::from(POOL_CONTRACT),
        mix_denom: TEST_MIX_DENOM.to_string(),
    };
    let env = mock_env();
    let info = mock_info("creator", &[]);
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
    deps
}

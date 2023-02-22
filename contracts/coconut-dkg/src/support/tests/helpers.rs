// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::instantiate;
use coconut_dkg_common::msg::InstantiateMsg;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier};
use cosmwasm_std::{Empty, MemoryStorage, OwnedDeps};

use super::fixtures::TEST_MIX_DENOM;

pub const ADMIN_ADDRESS: &str = "admin address";
pub const GROUP_CONTRACT: &str = "group contract address";
pub const MULTISIG_CONTRACT: &str = "multisig contract address";

pub fn init_contract() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
    let mut deps = mock_dependencies();
    let msg = InstantiateMsg {
        group_addr: String::from(GROUP_CONTRACT),
        multisig_addr: String::from(MULTISIG_CONTRACT),
        time_configuration: None,
        mix_denom: TEST_MIX_DENOM.to_string(),
    };
    let env = mock_env();
    let info = mock_info(ADMIN_ADDRESS, &[]);
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
    deps
}

// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::instantiate;
use coconut_dkg_common::msg::InstantiateMsg;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier};
use cosmwasm_std::{
    from_binary, to_binary, ContractResult, Empty, MemoryStorage, OwnedDeps, QuerierResult,
    SystemResult, WasmQuery,
};
use cw4::{Cw4QueryMsg, MemberResponse};
use lazy_static::lazy_static;
use std::sync::Mutex;

use super::fixtures::TEST_MIX_DENOM;

pub const ADMIN_ADDRESS: &str = "admin address";
pub const GROUP_CONTRACT: &str = "group contract address";
pub const MULTISIG_CONTRACT: &str = "multisig contract address";

lazy_static! {
    pub static ref GROUP_MEMBERS: Mutex<Vec<(String, u64)>> = Mutex::new(vec![]);
}

fn querier_handler(query: &WasmQuery) -> QuerierResult {
    let bin = match query {
        WasmQuery::Smart { contract_addr, msg } => {
            if contract_addr != GROUP_CONTRACT {
                panic!("Not supported");
            }
            let weight = match from_binary(msg) {
                Ok(Cw4QueryMsg::Member { addr, .. }) => GROUP_MEMBERS
                    .lock()
                    .unwrap()
                    .iter()
                    .find_map(|(a, w)| if *a == addr { Some(*w) } else { None }),
                _ => None,
            };
            let resp = MemberResponse { weight };
            to_binary(&resp).unwrap()
        }
        _ => panic!("Not supported"),
    };
    SystemResult::Ok(ContractResult::Ok(bin))
}

pub fn init_contract() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
    let mut deps = mock_dependencies();
    deps.querier.update_wasm(querier_handler);
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

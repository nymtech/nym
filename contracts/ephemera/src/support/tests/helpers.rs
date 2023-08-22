// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::instantiate;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier};
use cosmwasm_std::{
    from_binary, to_binary, ContractResult, Empty, MemoryStorage, OwnedDeps, QuerierResult,
    SystemResult, WasmQuery,
};
use cw4::{Cw4QueryMsg, Member, MemberListResponse, MemberResponse};
use lazy_static::lazy_static;
use nym_ephemera_common::msg::InstantiateMsg;
use std::sync::Mutex;

use super::fixtures::TEST_MIX_DENOM;

pub const ADMIN_ADDRESS: &str = "admin address";
pub const GROUP_CONTRACT: &str = "group contract address";

lazy_static! {
    pub static ref GROUP_MEMBERS: Mutex<Vec<(Member, u64)>> = Mutex::new(vec![]);
}

fn querier_handler(query: &WasmQuery) -> QuerierResult {
    let bin = match query {
        WasmQuery::Smart { contract_addr, msg } => {
            if contract_addr != GROUP_CONTRACT {
                panic!("Not supported");
            }
            match from_binary(msg) {
                Ok(Cw4QueryMsg::Member { addr, at_height }) => {
                    let weight = GROUP_MEMBERS.lock().unwrap().iter().find_map(|(m, h)| {
                        if m.addr == addr {
                            if let Some(height) = at_height {
                                if height != *h {
                                    return None;
                                }
                            }
                            Some(m.weight)
                        } else {
                            None
                        }
                    });
                    to_binary(&MemberResponse { weight }).unwrap()
                }
                Ok(Cw4QueryMsg::ListMembers { .. }) => {
                    let members = GROUP_MEMBERS
                        .lock()
                        .unwrap()
                        .iter()
                        .map(|m| m.0.clone())
                        .collect();
                    to_binary(&MemberListResponse { members }).unwrap()
                }
                _ => panic!("Not supported"),
            }
        }
        _ => panic!("Not supported"),
    };
    SystemResult::Ok(ContractResult::Ok(bin))
}

pub fn init_contract() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
    let mut deps = mock_dependencies();
    deps.querier.update_wasm(querier_handler);
    let msg = InstantiateMsg {
        group_addr: GROUP_CONTRACT.to_string(),
        mix_denom: TEST_MIX_DENOM.to_string(),
    };
    let env = mock_env();
    let info = mock_info(ADMIN_ADDRESS, &[]);
    instantiate(deps.as_mut(), env, info, msg).unwrap();
    deps
}

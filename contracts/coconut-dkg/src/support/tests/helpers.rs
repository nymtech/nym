// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::instantiate;
use crate::dealers::storage::current_dealers;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier};
use cosmwasm_std::{
    from_binary, to_binary, Addr, ContractResult, DepsMut, Empty, MemoryStorage, OwnedDeps,
    QuerierResult, SystemResult, WasmQuery,
};
use cw4::{Cw4QueryMsg, Member, MemberListResponse, MemberResponse};
use lazy_static::lazy_static;
use nym_coconut_dkg_common::msg::InstantiateMsg;
use nym_coconut_dkg_common::types::{DealerDetails, TOTAL_DEALINGS};
use std::sync::Mutex;

use super::fixtures::TEST_MIX_DENOM;

pub const ADMIN_ADDRESS: &str = "admin address";
pub const GROUP_CONTRACT: &str = "group contract address";
pub const MULTISIG_CONTRACT: &str = "multisig contract address";

lazy_static! {
    pub static ref GROUP_MEMBERS: Mutex<Vec<(Member, u64)>> = Mutex::new(vec![]);
}

pub fn add_fixture_dealer(deps: DepsMut<'_>) {
    let owner = Addr::unchecked("owner");
    current_dealers()
        .save(
            deps.storage,
            &owner,
            &DealerDetails {
                address: owner.clone(),
                bte_public_key_with_proof: String::new(),
                announce_address: String::new(),
                assigned_index: 100,
            },
        )
        .unwrap();
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
        group_addr: String::from(GROUP_CONTRACT),
        multisig_addr: String::from(MULTISIG_CONTRACT),
        time_configuration: None,
        mix_denom: TEST_MIX_DENOM.to_string(),
        key_size: TOTAL_DEALINGS as u32,
    };
    let env = mock_env();
    let info = mock_info(ADMIN_ADDRESS, &[]);
    instantiate(deps.as_mut(), env, info, msg).unwrap();
    deps
}

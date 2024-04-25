// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::instantiate;
use crate::dealers::storage::{DEALERS_INDICES, EPOCH_DEALERS_MAP};
use crate::epoch_state::storage::CURRENT_EPOCH;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier};
use cosmwasm_std::{
    from_binary, to_binary, Addr, ContractResult, DepsMut, Empty, MemoryStorage, OwnedDeps,
    QuerierResult, SystemResult, WasmQuery,
};
use cw4::{Cw4QueryMsg, Member, MemberListResponse, MemberResponse};
use nym_coconut_dkg_common::dealer::DealerRegistrationDetails;
use nym_coconut_dkg_common::dealing::DEFAULT_DEALINGS;
use nym_coconut_dkg_common::msg::InstantiateMsg;
use nym_coconut_dkg_common::types::{DealerDetails, EpochId};
use std::sync::Mutex;

use super::fixtures::TEST_MIX_DENOM;

pub const ADMIN_ADDRESS: &str = "admin address";
pub const GROUP_CONTRACT: &str = "group contract address";
pub const MULTISIG_CONTRACT: &str = "multisig contract address";

// wtf, why is this a thing?
pub(crate) static GROUP_MEMBERS: Mutex<Vec<(Member, u64)>> = Mutex::new(Vec::new());

pub fn re_register_dealer(deps: DepsMut, dealer: &Addr) {
    let epoch_id = CURRENT_EPOCH.load(deps.storage).unwrap().epoch_id;
    let previous = epoch_id - 1;
    let details = EPOCH_DEALERS_MAP
        .load(deps.storage, (previous, dealer))
        .unwrap();
    EPOCH_DEALERS_MAP
        .save(deps.storage, (epoch_id, dealer), &details)
        .unwrap()
}

pub fn add_current_dealer(deps: DepsMut<'_>, details: &DealerDetails) {
    let epoch_id = CURRENT_EPOCH.load(deps.storage).unwrap().epoch_id;
    insert_dealer(deps, epoch_id, details)
}

pub fn insert_dealer(deps: DepsMut<'_>, epoch_id: EpochId, details: &DealerDetails) {
    DEALERS_INDICES
        .save(deps.storage, &details.address, &details.assigned_index)
        .unwrap();

    EPOCH_DEALERS_MAP
        .save(
            deps.storage,
            (epoch_id, &details.address),
            &DealerRegistrationDetails {
                bte_public_key_with_proof: details.bte_public_key_with_proof.clone(),
                ed25519_identity: details.ed25519_identity.clone(),
                announce_address: details.announce_address.clone(),
            },
        )
        .unwrap()
}

pub fn add_fixture_dealer(deps: DepsMut<'_>) {
    let owner = Addr::unchecked("owner");
    add_current_dealer(
        deps,
        &DealerDetails {
            address: owner.clone(),
            bte_public_key_with_proof: String::new(),
            ed25519_identity: String::new(),
            announce_address: String::new(),
            assigned_index: 100,
        },
    );
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
        key_size: DEFAULT_DEALINGS as u32,
    };
    let env = mock_env();
    let info = mock_info(ADMIN_ADDRESS, &[]);
    instantiate(deps.as_mut(), env, info, msg).unwrap();
    deps
}

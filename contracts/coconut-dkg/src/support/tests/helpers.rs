// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::fixtures::TEST_MIX_DENOM;
use crate::contract::instantiate;
use crate::dealers::storage::{DEALERS_INDICES, EPOCH_DEALERS_MAP};
use crate::epoch_state::storage::{load_current_epoch, save_epoch};
use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, MockApi, MockQuerier};
use cosmwasm_std::{
    from_json, to_json_binary, Addr, ContractResult, DepsMut, Empty, MemoryStorage, OwnedDeps,
    QuerierResult, SystemResult, WasmQuery,
};
use cw4::{Cw4QueryMsg, Member, MemberListResponse, MemberResponse};
use easy_addr::addr;
use nym_coconut_dkg_common::dealer::DealerRegistrationDetails;
use nym_coconut_dkg_common::dealing::DEFAULT_DEALINGS;
use nym_coconut_dkg_common::msg::InstantiateMsg;
use nym_coconut_dkg_common::types::{DealerDetails, EpochId};

pub const ADMIN_ADDRESS: &str = addr!("admin address");
pub const GROUP_CONTRACT: &str = addr!("group contract address");
pub const MULTISIG_CONTRACT: &str = addr!("multisig contract address");

/// A test group member with the given weight.
pub fn group_member(addr: &str, weight: u64) -> Member {
    Member {
        addr: addr.to_string(),
        weight,
    }
}

pub fn re_register_dealer(deps: DepsMut, dealer: &Addr) {
    let epoch_id = load_current_epoch(deps.storage).unwrap().epoch_id;
    let previous = epoch_id - 1;
    let details = EPOCH_DEALERS_MAP
        .load(deps.storage, (previous, dealer))
        .unwrap();
    EPOCH_DEALERS_MAP
        .save(deps.storage, (epoch_id, dealer), &details)
        .unwrap()
}

pub fn add_current_dealer(deps: DepsMut<'_>, details: &DealerDetails) {
    let mut epoch = load_current_epoch(deps.storage).unwrap();
    let epoch_id = epoch.epoch_id;

    // mirror the real registration handler, which counts the dealer in the epoch's progress
    // as well as writing it to the dealer maps. a dealer present only in the maps is one the
    // contract cannot see when it decides whether a ceremony has anyone in it
    epoch.state_progress.registered_dealers += 1;
    save_epoch(deps.storage, mock_env().block.height, &epoch).unwrap();

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

/// Answer the group contract's queries the way cw4-group does, for a fixed set of members:
/// `ListMembers` pages in address order with the same default and maximum page size, so the
/// contract's own pagination gets exercised rather than handed everything at once.
#[allow(clippy::panic)]
fn group_querier(mut members: Vec<Member>) -> impl Fn(&WasmQuery) -> QuerierResult {
    const DEFAULT_LIMIT: usize = 10;
    const MAX_LIMIT: usize = 30;

    members.sort_by(|a, b| a.addr.cmp(&b.addr));

    move |query| {
        let bin = match query {
            WasmQuery::Smart { contract_addr, msg } => {
                if contract_addr != GROUP_CONTRACT {
                    panic!("Not supported");
                }
                match from_json(msg) {
                    Ok(Cw4QueryMsg::Member { addr, .. }) => {
                        let weight = members.iter().find(|m| m.addr == addr).map(|m| m.weight);
                        to_json_binary(&MemberResponse { weight }).unwrap()
                    }
                    Ok(Cw4QueryMsg::ListMembers { start_after, limit }) => {
                        let limit = limit
                            .map(|l| l as usize)
                            .unwrap_or(DEFAULT_LIMIT)
                            .min(MAX_LIMIT);
                        let page = members
                            .iter()
                            .filter(|m| start_after.as_ref().is_none_or(|after| &m.addr > after))
                            .take(limit)
                            .cloned()
                            .collect();
                        to_json_binary(&MemberListResponse { members: page }).unwrap()
                    }
                    _ => panic!("Not supported"),
                }
            }
            _ => panic!("Not supported"),
        };
        SystemResult::Ok(ContractResult::Ok(bin))
    }
}

/// Stand the contract up against a group with no members.
pub fn init_contract() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
    init_contract_with_group_members(Vec::new())
}

/// Stand the contract up against a group with exactly these members.
pub fn init_contract_with_group_members(
    members: Vec<Member>,
) -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
    let mut deps = mock_dependencies();
    deps.querier.update_wasm(group_querier(members));
    let msg = InstantiateMsg {
        group_addr: String::from(GROUP_CONTRACT),
        multisig_addr: String::from(MULTISIG_CONTRACT),
        time_configuration: None,
        mix_denom: TEST_MIX_DENOM.to_string(),
        key_size: DEFAULT_DEALINGS as u32,
    };
    let env = mock_env();
    let info = message_info(&Addr::unchecked(ADMIN_ADDRESS), &[]);
    instantiate(deps.as_mut(), env, info, msg).unwrap();
    deps
}

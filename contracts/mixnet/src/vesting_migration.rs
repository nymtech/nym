// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::delegations::storage as delegations_storage;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::helpers::get_mixnode_details_by_owner;
use crate::mixnodes::storage as mixnodes_storage;
use crate::support::helpers::{
    ensure_bonded, ensure_epoch_in_progress_state, ensure_no_pending_pledge_changes,
};
use cosmwasm_std::{wasm_execute, DepsMut, MessageInfo, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::{Delegation, NodeId};
use vesting_contract_common::messages::ExecuteMsg as VestingExecuteMsg;

pub(crate) fn try_migrate_vested_mixnode(
    deps: DepsMut<'_>,
    info: MessageInfo,
) -> Result<Response, MixnetContractError> {
    let mix_details = get_mixnode_details_by_owner(deps.storage, info.sender.clone())?.ok_or(
        MixnetContractError::NoAssociatedMixNodeBond {
            owner: info.sender.clone(),
        },
    )?;
    let mix_id = mix_details.mix_id();

    ensure_epoch_in_progress_state(deps.storage)?;
    ensure_no_pending_pledge_changes(&mix_details.pending_changes)?;
    ensure_bonded(&mix_details.bond_information)?;

    let Some(proxy) = &mix_details.bond_information.proxy else {
        return Err(MixnetContractError::NotAVestingMixnode);
    };

    let vesting_contract = mixnet_params_storage::vesting_contract_address(deps.storage)?;
    if proxy != vesting_contract {
        return Err(MixnetContractError::ProxyIsNotVestingContract {
            received: proxy.clone(),
            vesting_contract,
        });
    }

    let mut updated_bond = mix_details.bond_information.clone();
    updated_bond.proxy = None;
    mixnodes_storage::mixnode_bonds().replace(
        deps.storage,
        mix_id,
        Some(&updated_bond),
        Some(&mix_details.bond_information),
    )?;

    Ok(Response::new().add_message(wasm_execute(
        vesting_contract,
        &VestingExecuteMsg::TrackMigratedMixnode {
            owner: info.sender.into_string(),
        },
        vec![],
    )?))
}

pub(crate) fn try_migrate_vested_delegation(
    deps: DepsMut<'_>,
    info: MessageInfo,
    mix_id: NodeId,
) -> Result<Response, MixnetContractError> {
    ensure_epoch_in_progress_state(deps.storage)?;

    let vesting_contract = mixnet_params_storage::vesting_contract_address(deps.storage)?;

    let storage_key =
        Delegation::generate_storage_key(mix_id, &info.sender, Some(&vesting_contract));
    let Some(mut delegation) =
        delegations_storage::delegations().may_load(deps.storage, storage_key.clone())?
    else {
        return Err(MixnetContractError::NotAVestingDelegation);
    };

    // sanity check that's meant to blow up the contract
    assert_eq!(delegation.proxy, Some(vesting_contract.clone()));

    // update the delegation and save it under the correct storage key
    delegation.proxy = None;
    let updated_storage_key = Delegation::generate_storage_key(mix_id, &info.sender, None);
    delegations_storage::delegations().remove(deps.storage, storage_key)?;
    delegations_storage::delegations().save(deps.storage, updated_storage_key, &delegation)?;

    Ok(Response::new().add_message(wasm_execute(
        vesting_contract,
        &VestingExecuteMsg::TrackMigratedDelegation {
            owner: info.sender.into_string(),
            mix_id,
        },
        vec![],
    )?))
}

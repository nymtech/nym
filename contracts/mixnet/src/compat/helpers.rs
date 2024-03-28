// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::storage as mixnode_storage;
use crate::nodes::storage as nymnodes_storage;
use crate::support::helpers::ensure_epoch_in_progress_state;
use cosmwasm_std::{Coin, StdResult, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::helpers::{NodeBond, NodeDetails, PendingChanges};
use mixnet_contract_common::NodeId;

pub fn ensure_can_withdraw_rewards<D>(node_details: &D) -> Result<(), MixnetContractError>
where
    D: NodeDetails,
{
    // we can only withdraw rewards for a bonded node (i.e. not in the process of unbonding)
    // otherwise we know there are no rewards to withdraw
    node_details.bond_info().ensure_bonded()?;

    Ok(())
}

pub fn ensure_can_modify_cost_params<D>(
    storage: &dyn Storage,
    node_details: &D,
) -> Result<(), MixnetContractError>
where
    D: NodeDetails,
{
    // changing cost params is only allowed if the epoch is currently not in the process of being advanced
    ensure_epoch_in_progress_state(storage)?;

    // we can only change cost params for a bonded node (i.e. not in the process of unbonding)
    node_details.bond_info().ensure_bonded()?;

    Ok(())
}

fn ensure_can_modify_pledge<D>(
    storage: &dyn Storage,
    node_details: &D,
) -> Result<(), MixnetContractError>
where
    D: NodeDetails,
{
    // changing pledge is only allowed if the epoch is currently not in the process of being advanced
    ensure_epoch_in_progress_state(storage)?;

    // we can only change pledge for a bonded node (i.e. not in the process of unbonding)
    node_details.bond_info().ensure_bonded()?;

    // the node can't have any pending pledge changes
    node_details
        .pending_changes()
        .ensure_no_pending_pledge_changes()?;

    Ok(())
}

// remove duplicate code and make sure the same checks are performed everywhere
// (so nothing is accidentally missing)
pub fn ensure_can_increase_pledge<D>(
    storage: &dyn Storage,
    node_details: &D,
) -> Result<(), MixnetContractError>
where
    D: NodeDetails,
{
    ensure_can_modify_pledge(storage, node_details)
}

// remove duplicate code and make sure the same checks are performed everywhere
// (so nothing is accidentally missing)
pub fn ensure_can_decrease_pledge<D>(
    storage: &dyn Storage,
    node_details: &D,
    decrease_by: &Coin,
) -> Result<(), MixnetContractError>
where
    D: NodeDetails,
{
    ensure_can_modify_pledge(storage, node_details)?;

    let minimum_pledge = mixnet_params_storage::minimum_node_pledge(storage)?;

    // check that the denomination is correct
    if decrease_by.denom != minimum_pledge.denom {
        return Err(MixnetContractError::WrongDenom {
            received: decrease_by.denom.clone(),
            expected: minimum_pledge.denom,
        });
    }

    // also check if the request contains non-zero amount
    // (otherwise it's a no-op and we should we waste gas when resolving events?)
    if decrease_by.amount.is_zero() {
        return Err(MixnetContractError::ZeroCoinAmount);
    }

    // decreasing pledge can't result in the new pledge being lower than the minimum amount
    let new_pledge_amount = node_details
        .bond_info()
        .original_pledge()
        .amount
        .saturating_sub(decrease_by.amount);
    if new_pledge_amount < minimum_pledge.amount {
        return Err(MixnetContractError::InvalidPledgeReduction {
            current: node_details.bond_info().original_pledge().amount,
            decrease_by: decrease_by.amount,
            minimum: minimum_pledge.amount,
            denom: minimum_pledge.denom,
        });
    }

    Ok(())
}

pub fn get_bond(
    storage: &dyn Storage,
    node_id: NodeId,
) -> Result<Box<dyn NodeBond>, MixnetContractError> {
    if let Ok(mix_bond) = mixnode_storage::mixnode_bonds().load(storage, node_id) {
        Ok(Box::new(mix_bond))
    } else {
        let node_bond = nymnodes_storage::nym_nodes()
            .load(storage, node_id)
            .map_err(|_| MixnetContractError::NymNodeBondNotFound { node_id })?;
        Ok(Box::new(node_bond))
    }
}

pub fn may_get_bond(
    storage: &dyn Storage,
    node_id: NodeId,
) -> StdResult<Option<Box<dyn NodeBond>>> {
    if let Some(mix_bond) = mixnode_storage::mixnode_bonds().may_load(storage, node_id)? {
        Ok(Some(Box::new(mix_bond)))
    } else if let Some(node_bond) = nymnodes_storage::nym_nodes().may_load(storage, node_id)? {
        Ok(Some(Box::new(node_bond)))
    } else {
        Ok(None)
    }
}

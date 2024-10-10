// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::gateways::storage as gateways_storage;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::helpers::must_get_mixnode_bond_by_owner;
use crate::mixnodes::storage as mixnodes_storage;
use crate::nodes::helpers::must_get_node_bond_by_owner;
use crate::nodes::storage as nymnodes_storage;
use cosmwasm_std::{Addr, Coin, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::mixnode::PendingMixNodeChanges;
use mixnet_contract_common::{EpochState, EpochStatus, IdentityKeyRef, MixNodeBond, NodeId};
use nym_contracts_common::IdentityKey;
use nym_contracts_common::Percent;

pub(crate) fn validate_pledge(
    mut pledge: Vec<Coin>,
    minimum_pledge: Coin,
) -> Result<Coin, MixnetContractError> {
    // check if anything was put as bond
    if pledge.is_empty() {
        return Err(MixnetContractError::NoBondFound);
    }

    if pledge.len() > 1 {
        return Err(MixnetContractError::MultipleDenoms);
    }

    // check that the denomination is correct
    if pledge[0].denom != minimum_pledge.denom {
        return Err(MixnetContractError::WrongDenom {
            received: pledge[0].denom.clone(),
            expected: minimum_pledge.denom,
        });
    }

    // check that the pledge contains the minimum amount of tokens
    if pledge[0].amount < minimum_pledge.amount {
        return Err(MixnetContractError::InsufficientPledge {
            received: pledge[0].clone(),
            minimum: minimum_pledge,
        });
    }

    // throughout this function we've been using the value at `pledge[0]` without problems
    // (plus we have even validated that the vec is not empty), so the unwrap here is absolutely fine,
    // since it cannot possibly fail without UB
    #[allow(clippy::unwrap_used)]
    Ok(pledge.pop().unwrap())
}

pub(crate) fn validate_delegation_stake(
    mut delegation: Vec<Coin>,
    minimum_delegation: Option<Coin>,
    expected_denom: String,
) -> Result<Coin, MixnetContractError> {
    // check if anything was put as delegation
    if delegation.is_empty() {
        return Err(MixnetContractError::EmptyDelegation);
    }

    if delegation.len() > 1 {
        return Err(MixnetContractError::MultipleDenoms);
    }

    // check that the denomination is correct
    if delegation[0].denom != expected_denom {
        return Err(MixnetContractError::WrongDenom {
            received: delegation[0].denom.clone(),
            expected: expected_denom,
        });
    }

    // if we have a minimum set, check if enough tokens were sent, otherwise just check if its non-zero
    if let Some(minimum_delegation) = minimum_delegation {
        if delegation[0].amount < minimum_delegation.amount {
            return Err(MixnetContractError::InsufficientDelegation {
                received: delegation[0].clone(),
                minimum: minimum_delegation,
            });
        }
    } else if delegation[0].amount.is_zero() {
        return Err(MixnetContractError::EmptyDelegation);
    }

    // throughout this function we've been using the value at `delegation[0]` without problems
    // (plus we have even validated that the vec is not empty), so the unwrap here is absolutely fine,
    // since it cannot possibly fail without UB
    #[allow(clippy::unwrap_used)]
    Ok(delegation.pop().unwrap())
}

pub(crate) fn ensure_epoch_in_progress_state(
    storage: &dyn Storage,
) -> Result<(), MixnetContractError> {
    let epoch_status = crate::interval::storage::current_epoch_status(storage)?;
    if !matches!(epoch_status.state, EpochState::InProgress) {
        return Err(MixnetContractError::EpochAdvancementInProgress {
            current_state: epoch_status.state,
        });
    }
    Ok(())
}

pub(crate) fn ensure_is_authorized(
    sender: &Addr,
    storage: &dyn Storage,
) -> Result<(), MixnetContractError> {
    if sender != crate::mixnet_contract_settings::storage::rewarding_validator_address(storage)? {
        return Err(MixnetContractError::Unauthorized);
    }
    Ok(())
}

pub(crate) fn ensure_can_advance_epoch(
    sender: &Addr,
    storage: &dyn Storage,
) -> Result<EpochStatus, MixnetContractError> {
    let epoch_status = crate::interval::storage::current_epoch_status(storage)?;
    if sender != epoch_status.being_advanced_by {
        // well, we know we're going to throw an error now,
        // but we might as well also check if we're even a validator
        // to return a possibly better error message
        ensure_is_authorized(sender, storage)?;
        return Err(MixnetContractError::RewardingValidatorMismatch {
            current_validator: sender.clone(),
            chosen_validator: epoch_status.being_advanced_by,
        });
    }
    Ok(epoch_status)
}

pub(crate) fn ensure_bonded(bond: &MixNodeBond) -> Result<(), MixnetContractError> {
    if bond.is_unbonding {
        return Err(MixnetContractError::MixnodeIsUnbonding {
            mix_id: bond.mix_id,
        });
    }
    Ok(())
}

pub(crate) fn ensure_no_pending_pledge_changes(
    pending_changes: &PendingMixNodeChanges,
) -> Result<(), MixnetContractError> {
    if let Some(pending_event_id) = pending_changes.pledge_change {
        return Err(MixnetContractError::PendingPledgeChange { pending_event_id });
    }
    Ok(())
}

pub(crate) fn ensure_no_pending_params_changes(
    pending_changes: &PendingMixNodeChanges,
) -> Result<(), MixnetContractError> {
    if let Some(pending_event_id) = pending_changes.cost_params_change {
        return Err(MixnetContractError::PendingParamsChange { pending_event_id });
    }
    Ok(())
}

/// get identity key of the currently bonded legacy mixnode or nym-node
#[allow(dead_code)]
pub(crate) fn get_bond_identity(
    storage: &dyn Storage,
    owner: &Addr,
) -> Result<IdentityKey, MixnetContractError> {
    // legacy mixnode
    if let Ok(bond) = must_get_mixnode_bond_by_owner(storage, owner) {
        return Ok(bond.mix_node.identity_key);
    }
    // current nym-node
    must_get_node_bond_by_owner(storage, owner).map(|b| b.node.identity_key)
}

/// Checks whether a nym-node or a legacy mixnode with the provided id is currently bonded
pub(crate) fn ensure_any_node_bonded(
    storage: &dyn Storage,
    node_id: NodeId,
) -> Result<(), MixnetContractError> {
    // legacy mixnode
    if let Some(mixnode_bond) = mixnodes_storage::mixnode_bonds().may_load(storage, node_id)? {
        return if mixnode_bond.is_unbonding {
            Err(MixnetContractError::MixnodeIsUnbonding { mix_id: node_id })
        } else {
            Ok(())
        };
    }

    // current nym-node
    match nymnodes_storage::nym_nodes().may_load(storage, node_id)? {
        None => Err(MixnetContractError::NymNodeBondNotFound { node_id }),
        Some(bond) if bond.is_unbonding => Err(MixnetContractError::NodeIsUnbonding { node_id }),
        _ => Ok(()),
    }
}

// check if the target address has already bonded a mixnode or gateway,
// in either case, return an appropriate error
pub(crate) fn ensure_no_existing_bond(
    sender: &Addr,
    storage: &dyn Storage,
) -> Result<(), MixnetContractError> {
    if nymnodes_storage::nym_nodes()
        .idx
        .owner
        .item(storage, sender.clone())?
        .is_some()
    {
        return Err(MixnetContractError::AlreadyOwnsNymNode);
    }

    if mixnodes_storage::mixnode_bonds()
        .idx
        .owner
        .item(storage, sender.clone())?
        .is_some()
    {
        return Err(MixnetContractError::AlreadyOwnsMixnode);
    }

    if gateways_storage::gateways()
        .idx
        .owner
        .item(storage, sender.clone())?
        .is_some()
    {
        return Err(MixnetContractError::AlreadyOwnsGateway);
    }

    Ok(())
}

pub(crate) fn decode_ed25519_identity_key(
    encoded: IdentityKeyRef,
) -> Result<[u8; 32], MixnetContractError> {
    let mut public_key = [0u8; 32];
    let used = bs58::decode(encoded)
        .into(&mut public_key)
        .map_err(|err| MixnetContractError::MalformedEd25519IdentityKey(err.to_string()))?;

    if used != 32 {
        return Err(MixnetContractError::MalformedEd25519IdentityKey(
            "Too few bytes provided for the public key".into(),
        ));
    }

    Ok(public_key)
}

pub(crate) fn ensure_profit_margin_within_range(
    storage: &dyn Storage,
    profit_margin: Percent,
) -> Result<(), MixnetContractError> {
    let range = mixnet_params_storage::profit_margin_range(storage)?;
    if !range.within_range(profit_margin) {
        return Err(MixnetContractError::ProfitMarginOutsideRange {
            provided: profit_margin,
            range,
        });
    }

    Ok(())
}

pub fn ensure_operating_cost_within_range(
    storage: &dyn Storage,
    operating_cost: &Coin,
) -> Result<(), MixnetContractError> {
    let range = mixnet_params_storage::interval_operating_cost_range(storage)?;
    if !range.within_range(operating_cost.amount) {
        return Err(MixnetContractError::OperatingCostOutsideRange {
            denom: operating_cost.denom.clone(),
            provided: operating_cost.amount,
            range,
        });
    }

    Ok(())
}

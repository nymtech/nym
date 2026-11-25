// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, BankMsg, Coin, DepsMut, Env, Response, Uint128, Uint256};

use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_active_set_update_event, new_active_set_update_failure, new_cost_params_update_event,
    new_delegation_event, new_delegation_on_unbonded_node_event, new_mixnode_unbonding_event,
    new_nym_node_unbonding_event, new_pledge_decrease_event, new_pledge_increase_event,
    new_redelegation_event, new_redelegation_failed_event, new_rewarding_params_update_event,
    new_undelegation_event,
};
use mixnet_contract_common::mixnode::NodeCostParams;
use mixnet_contract_common::msg::{RedelegationTarget, MAX_REDELEGATION_TARGETS};
use mixnet_contract_common::pending_events::{
    PendingEpochEventData, PendingEpochEventKind, PendingIntervalEventData,
    PendingIntervalEventKind,
};
use mixnet_contract_common::reward_params::{ActiveSetUpdate, IntervalRewardingParamsUpdate};
use mixnet_contract_common::rewarding::helpers::truncate_reward;
use mixnet_contract_common::{BlockHeight, Delegation, NodeId};
use nym_contracts_common::helpers::ResponseExt;

use crate::delegations;
use crate::delegations::storage as delegations_storage;
use crate::interval::helpers::change_interval_config;
use crate::interval::storage;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::helpers::{cleanup_post_unbond_mixnode_storage, get_mixnode_details_by_id};
use crate::mixnodes::storage as mixnodes_storage;
use crate::nodes::helpers::{cleanup_post_unbond_nym_node_storage, get_node_details_by_id};
use crate::nodes::storage as nymnodes_storage;
use crate::rewards::storage as rewards_storage;
use crate::rewards::storage::RewardingStorage;
use crate::support::helpers::ensure_any_node_bonded;

pub(crate) trait ContractExecutableEvent {
    // note: the error only means a HARD error like we failed to read from storage.
    // if, for example, delegating fails because mixnode no longer exists, we return an Ok(()),
    // because it's not a hard error and we don't want to fail the entire transaction
    fn execute(self, deps: DepsMut<'_>, env: &Env) -> Result<Response, MixnetContractError>;
}

pub(crate) fn delegate(
    deps: DepsMut<'_>,
    env: &Env,
    created_at: BlockHeight,
    owner: Addr,
    node_id: NodeId,
    amount: Coin,
) -> Result<Response, MixnetContractError> {
    // if the node has fully unbonded (no rewarding info) or is mid-unbonding, a plain delegation
    // has nowhere to land, so return the tokens to the owner's wallet
    if rewards_storage::NYMNODE_REWARDING
        .may_load(deps.storage, node_id)?
        .is_none()
    {
        return Ok(Response::new()
            .send_tokens(&owner, amount)
            .add_event(new_delegation_on_unbonded_node_event(&owner, node_id)));
    }

    // the underlying node might have unbonded between this event getting created and executed;
    // we have to check both nym-node and legacy nym-node
    match ensure_any_node_bonded(deps.storage, node_id) {
        // cosmwasm-std errors are critical because they imply issues with the underlying storage
        Err(MixnetContractError::StdErr { source }) => return Err(source.into()),
        Err(_unbonded) => {
            return Ok(Response::new()
                .send_tokens(&owner, amount)
                .add_event(new_delegation_on_unbonded_node_event(&owner, node_id)));
        }
        Ok(_) => {}
    }

    delegate_to_bonded_node(deps, env, created_at, owner, node_id, amount)
}

/// Places `amount` on a bonded node, compounding any existing delegation.
/// The caller must verify that bonding and rewarding data exist.
pub(crate) fn delegate_to_bonded_node(
    deps: DepsMut<'_>,
    env: &Env,
    created_at: BlockHeight,
    owner: Addr,
    node_id: NodeId,
    amount: Coin,
) -> Result<Response, MixnetContractError> {
    let mut node_rewarding = rewards_storage::NYMNODE_REWARDING
        .may_load(deps.storage, node_id)?
        .ok_or(MixnetContractError::inconsistent_state(
            "delegating to a node with no rewarding info despite it being checked as bonded",
        ))?;

    let new_delegation_amount = amount.clone();

    // if there's an existing delegation, withdraw its full reward and create a fresh delegation
    // with the sum of both (the compound)
    let mut stored_delegation_amount = amount;
    let storage_key = Delegation::generate_storage_key(node_id, &owner, None);
    let old_delegation = if let Some(existing_delegation) =
        delegations_storage::delegations().may_load(deps.storage, storage_key.clone())?
    {
        let og_with_reward = node_rewarding.undelegate(&existing_delegation)?;
        stored_delegation_amount.amount += og_with_reward.amount;
        Some(existing_delegation)
    } else {
        None
    };

    node_rewarding.add_base_delegation(stored_delegation_amount.amount)?;

    let cosmos_event = new_delegation_event(
        created_at,
        &owner,
        &new_delegation_amount,
        node_id,
        node_rewarding.total_unit_reward,
    );

    let delegation = Delegation::new(
        owner,
        node_id,
        node_rewarding.total_unit_reward,
        stored_delegation_amount,
        env.block.height,
    );

    delegations_storage::delegations().replace(
        deps.storage,
        storage_key,
        Some(&delegation),
        old_delegation.as_ref(),
    )?;
    rewards_storage::NYMNODE_REWARDING.save(deps.storage, node_id, &node_rewarding)?;

    Ok(Response::new().add_event(cosmos_event))
}

pub(crate) fn undelegate(
    deps: DepsMut<'_>,
    created_at: BlockHeight,
    owner: Addr,
    mix_id: NodeId,
) -> Result<Response, MixnetContractError> {
    // see if the delegation still exists (in case of impatient user who decided to send multiple
    // undelegation requests in an epoch)
    let storage_key = Delegation::generate_storage_key(mix_id, &owner, None);
    let delegation = match delegations_storage::delegations().may_load(deps.storage, storage_key)? {
        None => return Ok(Response::default()),
        Some(delegation) => delegation,
    };

    // note: rewarding information for nym-nodes and mixnodes are stored under the same storage structure
    // and in the same underlying map so it doesn't matter which one we load

    let rewarding_info =
        rewards_storage::NYMNODE_REWARDING.may_load(deps.storage, mix_id)?.ok_or(MixnetContractError::inconsistent_state(
            "node rewarding got removed from the storage whilst there's still an existing delegation",
        ))?;
    // this also appropriately adjusts the storage
    let tokens_to_return =
        delegations::helpers::undelegate(deps.storage, delegation, rewarding_info)?;

    let response = Response::new()
        .send_tokens(&owner, tokens_to_return.clone())
        .add_event(new_undelegation_event(created_at, &owner, mix_id));

    Ok(response)
}

pub(crate) fn redelegate(
    deps: DepsMut<'_>,
    env: &Env,
    created_at: BlockHeight,
    owner: Addr,
    from_node_id: NodeId,
    targets: Vec<RedelegationTarget>,
) -> Result<Response, MixnetContractError> {
    let mut deps = deps;

    // Validate stored events defensively so malformed input cannot block reconciliation.
    if let Err(reason) = validate_redelegation_shape(&targets, from_node_id) {
        return Ok(Response::new().add_event(new_redelegation_failed_event(
            created_at,
            &owner,
            from_node_id,
            reason,
        )));
    }

    // An earlier event may already have removed the source delegation.
    let storage_key = Delegation::generate_storage_key(from_node_id, &owner, None);
    let Some(delegation) =
        delegations_storage::delegations().may_load(deps.storage, storage_key)?
    else {
        return Ok(Response::new().add_event(new_redelegation_failed_event(
            created_at,
            &owner,
            from_node_id,
            "source delegation no longer exists",
        )));
    };

    // Nym nodes and legacy mixnodes share the rewarding map.
    let from_rewarding = rewards_storage::NYMNODE_REWARDING
        .may_load(deps.storage, from_node_id)?
        .ok_or(MixnetContractError::inconsistent_state(
            "node rewarding got removed from the storage whilst there's still an existing delegation",
        ))?;

    let denom = delegation.amount.denom.clone();

    // Calculate the same amount as NodeRewarding::undelegate without changing state.
    let settled = {
        let reward = from_rewarding.determine_delegation_reward(&delegation)?;
        let full = reward + delegation.dec_amount()?;
        truncate_reward(full, &denom).amount
    };

    // Complete all fallible availability checks before removing the source.
    for target in &targets {
        match ensure_any_node_bonded(deps.storage, target.to_node_id) {
            Ok(()) => {}
            Err(
                MixnetContractError::MixnodeIsUnbonding { .. }
                | MixnetContractError::NymNodeBondNotFound { .. }
                | MixnetContractError::NodeIsUnbonding { .. },
            ) => {
                let reason = format!("target node {} is no longer bonded", target.to_node_id);
                return Ok(Response::new().add_event(new_redelegation_failed_event(
                    created_at,
                    &owner,
                    from_node_id,
                    &reason,
                )));
            }
            Err(err) => return Err(err),
        }

        if rewards_storage::NYMNODE_REWARDING
            .may_load(deps.storage, target.to_node_id)?
            .is_none()
        {
            let reason = format!(
                "target node {} has no rewarding information",
                target.to_node_id
            );
            return Ok(Response::new().add_event(new_redelegation_failed_event(
                created_at,
                &owner,
                from_node_id,
                &reason,
            )));
        }
    }

    let shares = split_by_weight(settled, &targets)?;

    let minimum = mixnet_params_storage::CONTRACT_STATE
        .load(deps.storage)?
        .params
        .delegations_params
        .minimum_delegation;
    for share in &shares {
        if share.is_zero() {
            return Ok(Response::new().add_event(new_redelegation_failed_event(
                created_at,
                &owner,
                from_node_id,
                "a split share is zero",
            )));
        }
        if minimum.as_ref().is_some_and(|min| *share < min.amount) {
            return Ok(Response::new().add_event(new_redelegation_failed_event(
                created_at,
                &owner,
                from_node_id,
                "a split share is below the minimum delegation",
            )));
        }
    }

    let moved = delegations::helpers::undelegate(deps.storage, delegation, from_rewarding)?;
    if moved.amount != settled {
        return Err(MixnetContractError::inconsistent_state(
            "settled redelegation amount changed between preflight and execution",
        ));
    }

    let mut response = Response::new();
    for (target, share) in targets.iter().zip(shares.iter()) {
        let coin = Coin {
            denom: denom.clone(),
            amount: *share,
        };
        let sub = delegate_to_bonded_node(
            deps.branch(),
            env,
            created_at,
            owner.clone(),
            target.to_node_id,
            coin.clone(),
        )?;
        response = response
            .add_submessages(sub.messages)
            .add_events(sub.events)
            .add_event(new_redelegation_event(
                created_at,
                &owner,
                from_node_id,
                target.to_node_id,
                &coin,
            ));
    }

    Ok(response)
}

fn validate_redelegation_shape(
    targets: &[RedelegationTarget],
    from_node_id: NodeId,
) -> Result<(), &'static str> {
    if targets.is_empty() {
        return Err("no redelegation targets");
    }
    if targets.len() > MAX_REDELEGATION_TARGETS {
        return Err("too many redelegation targets");
    }
    if targets.len() == 1 && targets[0].to_node_id == from_node_id {
        return Err("single target equal to source");
    }
    let mut seen = Vec::with_capacity(targets.len());
    for target in targets {
        if target.weight == 0 {
            return Err("zero target weight");
        }
        if seen.contains(&target.to_node_id) {
            return Err("duplicate target");
        }
        seen.push(target.to_node_id);
    }
    Ok(())
}

/// Splits `total` by weight. Equal remainders are ordered by node id.
fn split_by_weight(
    total: Uint128,
    targets: &[RedelegationTarget],
) -> Result<Vec<Uint128>, MixnetContractError> {
    if targets.is_empty() || targets.iter().any(|target| target.weight == 0) {
        return Err(MixnetContractError::inconsistent_state(
            "invalid targets passed to redelegation split",
        ));
    }
    let total_weight = targets.iter().try_fold(0u128, |sum, target| {
        sum.checked_add(target.weight as u128).ok_or_else(|| {
            MixnetContractError::inconsistent_state("redelegation weight sum overflowed")
        })
    })?;
    let denom = Uint256::from(total_weight);

    let mut shares = Vec::with_capacity(targets.len());
    let mut remainders = Vec::with_capacity(targets.len());
    let mut allocated = Uint128::zero();
    for (i, target) in targets.iter().enumerate() {
        let numerator = total.full_mul(target.weight);
        let floor256 = numerator / denom;
        let floor = Uint128::try_from(floor256).map_err(|_| {
            MixnetContractError::inconsistent_state("redelegation floor share overflowed u128")
        })?;
        let remainder = numerator - floor256 * denom;
        allocated = allocated.checked_add(floor).map_err(|_| {
            MixnetContractError::inconsistent_state("redelegation allocation overflowed")
        })?;
        shares.push(floor);
        remainders.push((remainder, target.to_node_id, i));
    }

    let mut leftover = total.checked_sub(allocated).map_err(|_| {
        MixnetContractError::inconsistent_state(
            "redelegation allocated more than the settled amount",
        )
    })?;
    remainders.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut rank = 0usize;
    while !leftover.is_zero() {
        let Some((_, _, idx)) = remainders.get(rank) else {
            return Err(MixnetContractError::inconsistent_state(
                "redelegation leftover exceeded target count",
            ));
        };
        shares[*idx] = shares[*idx].checked_add(Uint128::one()).map_err(|_| {
            MixnetContractError::inconsistent_state("redelegation share overflowed")
        })?;
        leftover = leftover.checked_sub(Uint128::one()).map_err(|_| {
            MixnetContractError::inconsistent_state("redelegation leftover underflow")
        })?;
        rank += 1;
    }

    Ok(shares)
}

pub(crate) fn unbond_nym_node(
    deps: DepsMut<'_>,
    env: &Env,
    created_at: BlockHeight,
    node_id: NodeId,
) -> Result<Response, MixnetContractError> {
    // if we're here it means user executed `try_remove_nym_node` and as a result node was set to be
    // in unbonding state and thus nothing could have been done to it (such as attempting to double unbond it)
    // thus the node with all its associated information MUST exist in the storage.
    let node_details = get_node_details_by_id(deps.storage, node_id)?.ok_or(
        MixnetContractError::inconsistent_state(
            "nym node getting processed to get unbonded doesn't exist in the storage",
        ),
    )?;
    if node_details.pending_changes.pledge_change.is_some() {
        return Err(MixnetContractError::inconsistent_state(
            "attempted to unbond nym node while there are associated pending pledge changes",
        ));
    }

    // the denom on the original pledge was validated at the time of bonding, so we can safely reuse it here
    let rewarding_denom = &node_details.bond_information.original_pledge.denom;
    let tokens = node_details
        .rewarding_details
        .operator_pledge_with_reward(rewarding_denom);

    let owner = &node_details.bond_information.owner;

    // send bonded funds (alongside all earned rewards) to the bond owner
    let return_tokens = BankMsg::Send {
        to_address: owner.to_string(),
        amount: vec![tokens],
    };

    // remove the bond and if there are no delegations left, also the rewarding information
    cleanup_post_unbond_nym_node_storage(deps.storage, env, &node_details)?;

    let response = Response::new()
        .add_message(return_tokens)
        .add_event(new_nym_node_unbonding_event(created_at, node_id));

    Ok(response)
}

pub(crate) fn unbond_mixnode(
    deps: DepsMut<'_>,
    env: &Env,
    created_at: BlockHeight,
    mix_id: NodeId,
) -> Result<Response, MixnetContractError> {
    // if we're here it means user executed `_try_remove_mixnode` and as a result node was set to be
    // in unbonding state and thus nothing could have been done to it (such as attempting to double unbond it)
    // thus the node with all its associated information MUST exist in the storage.
    let node_details = get_mixnode_details_by_id(deps.storage, mix_id)?.ok_or(
        MixnetContractError::inconsistent_state(
            "mixnode getting processed to get unbonded doesn't exist in the storage",
        ),
    )?;
    if node_details.pending_changes.pledge_change.is_some() {
        return Err(MixnetContractError::inconsistent_state(
            "attempted to unbond mixnode while there are associated pending pledge changes",
        ));
    }

    // the denom on the original pledge was validated at the time of bonding so we can safely reuse it here
    let rewarding_denom = &node_details.bond_information.original_pledge.denom;
    let tokens = node_details
        .rewarding_details
        .operator_pledge_with_reward(rewarding_denom);

    let owner = &node_details.bond_information.owner;

    // remove the bond and if there are no delegations left, also the rewarding information
    // decrement the associated layer count
    cleanup_post_unbond_mixnode_storage(deps.storage, env, &node_details)?;

    let response = Response::new()
        .send_tokens(owner, tokens.clone())
        .add_event(new_mixnode_unbonding_event(created_at, mix_id));

    Ok(response)
}

pub(crate) fn update_active_set(
    deps: DepsMut<'_>,
    created_at: BlockHeight,
    update: ActiveSetUpdate,
) -> Result<Response, MixnetContractError> {
    // We don't have to check for authorization as this event can only be pushed
    // by the authorized entity.
    // Furthermore, we don't need to check whether the epoch is finished as the
    // queue is only emptied upon the epoch finishing.
    let global_rewarding_params_storage = RewardingStorage::load().global_rewarding_params;
    let mut rewarding_params = global_rewarding_params_storage.load(deps.storage)?;

    // unfortunately this will be a silent error,
    // but for this to happen an admin must have been screwing around by pushing active set update onto the queue
    // but updating rewarded set immediately, so it's on them
    if let Err(err) = rewarding_params.try_change_active_set(update) {
        return Ok(Response::new().add_event(new_active_set_update_failure(err)));
    }
    global_rewarding_params_storage.save(deps.storage, &rewarding_params)?;

    Ok(Response::new().add_event(new_active_set_update_event(created_at, update)))
}

pub(crate) fn increase_nym_node_pledge(
    deps: DepsMut<'_>,
    created_at: BlockHeight,
    node_id: NodeId,
    increase: Coin,
) -> Result<Response, MixnetContractError> {
    // note: we have already validated the amount to know it has the correct denomination

    // the target node MUST exist - we have checked it at the time of putting this event onto the queue
    // we have also verified there were no preceding unbond events
    let node_details = get_node_details_by_id(deps.storage, node_id)?.ok_or(
        MixnetContractError::inconsistent_state(
            "nym node getting processed to increase its pledge doesn't exist in the storage",
        ),
    )?;
    if node_details.pending_changes.pledge_change.is_none() {
        return Err(MixnetContractError::inconsistent_state(
            "attempted to increase nym node pledge while there are no associated pending changes",
        ));
    }

    let mut updated_bond = node_details.bond_information.clone();
    let mut updated_rewarding = node_details.rewarding_details;

    updated_bond.original_pledge.amount += increase.amount;
    updated_rewarding.increase_operator_uint128(increase.amount)?;

    let mut pending_changes = node_details.pending_changes;
    pending_changes.pledge_change = None;

    // update all: bond information, rewarding details and pending pledge changes
    nymnodes_storage::nym_nodes().replace(
        deps.storage,
        node_id,
        Some(&updated_bond),
        Some(&node_details.bond_information),
    )?;
    rewards_storage::NYMNODE_REWARDING.save(deps.storage, node_id, &updated_rewarding)?;
    nymnodes_storage::PENDING_NYMNODE_CHANGES.save(deps.storage, node_id, &pending_changes)?;

    Ok(Response::new().add_event(new_pledge_increase_event(created_at, node_id, &increase)))
}

pub(crate) fn increase_mixnode_pledge(
    deps: DepsMut<'_>,
    created_at: BlockHeight,
    mix_id: NodeId,
    increase: Coin,
) -> Result<Response, MixnetContractError> {
    // note: we have already validated the amount to know it has the correct denomination

    // the target node MUST exist - we have checked it at the time of putting this event onto the queue
    // we have also verified there were no preceding unbond events
    let mix_details = get_mixnode_details_by_id(deps.storage, mix_id)?.ok_or(
        MixnetContractError::inconsistent_state(
            "mixnode getting processed to increase its pledge doesn't exist in the storage",
        ),
    )?;
    if mix_details.pending_changes.pledge_change.is_none() {
        return Err(MixnetContractError::inconsistent_state(
            "attempted to increase mixnode pledge while there are no associated pending changes",
        ));
    }

    let mut updated_bond = mix_details.bond_information.clone();
    let mut updated_rewarding = mix_details.rewarding_details;

    updated_bond.original_pledge.amount += increase.amount;
    updated_rewarding.increase_operator_uint128(increase.amount)?;

    let mut pending_changes = mix_details.pending_changes;
    pending_changes.pledge_change = None;

    // update all: bond information, rewarding details and pending pledge changes
    mixnodes_storage::mixnode_bonds().replace(
        deps.storage,
        mix_id,
        Some(&updated_bond),
        Some(&mix_details.bond_information),
    )?;
    rewards_storage::MIXNODE_REWARDING.save(deps.storage, mix_id, &updated_rewarding)?;
    mixnodes_storage::PENDING_MIXNODE_CHANGES.save(deps.storage, mix_id, &pending_changes)?;

    Ok(Response::new().add_event(new_pledge_increase_event(created_at, mix_id, &increase)))
}

pub(crate) fn decrease_mixnode_pledge(
    deps: DepsMut<'_>,
    created_at: BlockHeight,
    mix_id: NodeId,
    decrease_by: Coin,
) -> Result<Response, MixnetContractError> {
    // the target node MUST exist - we have checked it at the time of putting this event onto the queue
    // we have also verified there were no preceding unbond events
    let mix_details = get_mixnode_details_by_id(deps.storage, mix_id)?.ok_or(
        MixnetContractError::inconsistent_state(
            "mixnode getting processed to increase its pledge doesn't exist in the storage",
        ),
    )?;
    if mix_details.pending_changes.pledge_change.is_none() {
        return Err(MixnetContractError::inconsistent_state(
            "attempted to decrease mixnode pledge while there are no associated pending changes",
        ));
    }

    let mut updated_bond = mix_details.bond_information.clone();
    let mut updated_rewarding = mix_details.rewarding_details;

    let mut pending_changes = mix_details.pending_changes;
    pending_changes.pledge_change = None;

    // SAFETY: the subtraction here can't overflow as before the event was pushed into the queue,
    // we checked that the new value will be higher than minimum pledge (which is also strictly positive)
    updated_bond.original_pledge.amount -= decrease_by.amount;
    updated_rewarding.decrease_operator_uint128(decrease_by.amount)?;

    let owner = &mix_details.bond_information.owner;

    // update all: bond information, rewarding details and pending pledge changes
    mixnodes_storage::mixnode_bonds().replace(
        deps.storage,
        mix_id,
        Some(&updated_bond),
        Some(&mix_details.bond_information),
    )?;
    rewards_storage::MIXNODE_REWARDING.save(deps.storage, mix_id, &updated_rewarding)?;
    mixnodes_storage::PENDING_MIXNODE_CHANGES.save(deps.storage, mix_id, &pending_changes)?;

    let response = Response::new()
        .send_tokens(owner, decrease_by.clone())
        .add_event(new_pledge_decrease_event(created_at, mix_id, &decrease_by));

    Ok(response)
}

pub(crate) fn decrease_nym_node_pledge(
    deps: DepsMut<'_>,
    created_at: BlockHeight,
    node_id: NodeId,
    decrease_by: Coin,
) -> Result<Response, MixnetContractError> {
    // the target node MUST exist - we have checked it at the time of putting this event onto the queue
    // we have also verified there were no preceding unbond events
    let node_details = get_node_details_by_id(deps.storage, node_id)?.ok_or(
        MixnetContractError::inconsistent_state(
            "nym node getting processed to increase its pledge doesn't exist in the storage",
        ),
    )?;
    if node_details.pending_changes.pledge_change.is_none() {
        return Err(MixnetContractError::inconsistent_state(
            "attempted to increase nym node pledge while there are no associated pending changes",
        ));
    }

    let mut updated_bond = node_details.bond_information.clone();
    let mut updated_rewarding = node_details.rewarding_details;

    let mut pending_changes = node_details.pending_changes;
    pending_changes.pledge_change = None;

    // SAFETY: the subtraction here can't overflow as before the event was pushed into the queue,
    // we checked that the new value will be higher than minimum pledge (which is also strictly positive)
    updated_bond.original_pledge.amount -= decrease_by.amount;
    updated_rewarding.decrease_operator_uint128(decrease_by.amount)?;

    let owner = &node_details.bond_information.owner;

    // send the removed tokens back to the operator
    let return_tokens = BankMsg::Send {
        to_address: owner.to_string(),
        amount: vec![decrease_by.clone()],
    };

    // update all: bond information, rewarding details and pending pledge changes
    nymnodes_storage::nym_nodes().replace(
        deps.storage,
        node_id,
        Some(&updated_bond),
        Some(&node_details.bond_information),
    )?;
    rewards_storage::NYMNODE_REWARDING.save(deps.storage, node_id, &updated_rewarding)?;
    nymnodes_storage::PENDING_NYMNODE_CHANGES.save(deps.storage, node_id, &pending_changes)?;

    let response = Response::new()
        .add_message(return_tokens)
        .add_event(new_pledge_decrease_event(created_at, node_id, &decrease_by));

    Ok(response)
}

impl ContractExecutableEvent for PendingEpochEventData {
    fn execute(self, deps: DepsMut<'_>, env: &Env) -> Result<Response, MixnetContractError> {
        // note that the basic validation on all those events was already performed before
        // they were pushed onto the queue
        match self.kind {
            PendingEpochEventKind::Delegate {
                owner,
                node_id: mix_id,
                amount,
                ..
            } => delegate(deps, env, self.created_at, owner, mix_id, amount),
            PendingEpochEventKind::Undelegate {
                owner,
                node_id: mix_id,
                ..
            } => undelegate(deps, self.created_at, owner, mix_id),
            PendingEpochEventKind::Redelegate {
                owner,
                from_node_id,
                targets,
                ..
            } => redelegate(deps, env, self.created_at, owner, from_node_id, targets),
            PendingEpochEventKind::NymNodePledgeMore { node_id, amount } => {
                increase_nym_node_pledge(deps, self.created_at, node_id, amount)
            }
            PendingEpochEventKind::MixnodePledgeMore { mix_id, amount } => {
                increase_mixnode_pledge(deps, self.created_at, mix_id, amount)
            }
            PendingEpochEventKind::NymNodeDecreasePledge {
                node_id,
                decrease_by,
            } => decrease_nym_node_pledge(deps, self.created_at, node_id, decrease_by),
            PendingEpochEventKind::MixnodeDecreasePledge {
                mix_id,
                decrease_by,
            } => decrease_mixnode_pledge(deps, self.created_at, mix_id, decrease_by),
            PendingEpochEventKind::UnbondMixnode { mix_id } => {
                unbond_mixnode(deps, env, self.created_at, mix_id)
            }
            PendingEpochEventKind::UnbondNymNode { node_id } => {
                unbond_nym_node(deps, env, self.created_at, node_id)
            }
            PendingEpochEventKind::UpdateActiveSet { update } => {
                update_active_set(deps, self.created_at, update)
            }
        }
    }
}

pub(crate) fn change_mix_cost_params(
    deps: DepsMut<'_>,
    created_at: BlockHeight,
    mix_id: NodeId,
    new_costs: NodeCostParams,
) -> Result<Response, MixnetContractError> {
    // almost an entire interval might have passed since the request was issued -> check if the
    // node still exists
    //
    // note: there's no check if the bond is in "unbonding" state, as epoch actions would get
    // cleared before touching interval actions
    let mut mix_rewarding =
        match rewards_storage::MIXNODE_REWARDING.may_load(deps.storage, mix_id)? {
            Some(mix_rewarding) if mix_rewarding.still_bonded() => mix_rewarding,
            // if node doesn't exist anymore, don't do anything, simple as that.
            _ => return Ok(Response::default()),
        };

    let mut pending_changes =
        mixnodes_storage::PENDING_MIXNODE_CHANGES.load(deps.storage, mix_id)?;
    pending_changes.cost_params_change = None;

    let cosmos_event = new_cost_params_update_event(created_at, mix_id, &new_costs);

    // TODO: can we just change cost_params without breaking rewarding calculation?
    // (I'm almost certain we can, but well, it has to be tested)
    mix_rewarding.cost_params = new_costs;
    rewards_storage::MIXNODE_REWARDING.save(deps.storage, mix_id, &mix_rewarding)?;
    mixnodes_storage::PENDING_MIXNODE_CHANGES.save(deps.storage, mix_id, &pending_changes)?;

    Ok(Response::new().add_event(cosmos_event))
}

pub(crate) fn change_nym_node_cost_params(
    deps: DepsMut<'_>,
    created_at: BlockHeight,
    node_id: NodeId,
    new_costs: NodeCostParams,
) -> Result<Response, MixnetContractError> {
    // almost an entire interval might have passed since the request was issued -> check if the
    // node still exists
    //
    // note: there's no check if the bond is in "unbonding" state, as epoch actions would get
    // cleared before touching interval actions
    let mut node_rewarding =
        match rewards_storage::NYMNODE_REWARDING.may_load(deps.storage, node_id)? {
            Some(node_rewarding) if node_rewarding.still_bonded() => node_rewarding,
            // if node doesn't exist anymore, don't do anything, simple as that.
            _ => return Ok(Response::default()),
        };

    let mut pending_changes =
        nymnodes_storage::PENDING_NYMNODE_CHANGES.load(deps.storage, node_id)?;
    pending_changes.cost_params_change = None;

    let cosmos_event = new_cost_params_update_event(created_at, node_id, &new_costs);

    node_rewarding.cost_params = new_costs;
    rewards_storage::NYMNODE_REWARDING.save(deps.storage, node_id, &node_rewarding)?;
    nymnodes_storage::PENDING_NYMNODE_CHANGES.save(deps.storage, node_id, &pending_changes)?;

    Ok(Response::new().add_event(cosmos_event))
}

pub(crate) fn update_rewarding_params(
    deps: DepsMut<'_>,
    created_at: BlockHeight,
    updated_params: IntervalRewardingParamsUpdate,
) -> Result<Response, MixnetContractError> {
    // We don't have to check for authorization as this event can only be pushed
    // by the authorized entity.
    // Furthermore, we don't need to check whether the interval is finished as the
    // queue is only emptied upon the interval finishing.
    // Also, we know the update is valid as we checked for that before pushing the event onto the queue.
    let interval = storage::current_interval(deps.storage)?;

    let mut rewarding_params = rewards_storage::REWARDING_PARAMS.load(deps.storage)?;
    rewarding_params.try_apply_updates(updated_params, interval.epochs_in_interval())?;
    rewards_storage::REWARDING_PARAMS.save(deps.storage, &rewarding_params)?;

    Ok(Response::new().add_event(new_rewarding_params_update_event(
        created_at,
        updated_params,
        rewarding_params.interval,
    )))
}

pub(crate) fn update_interval_config(
    deps: DepsMut,
    created_at: BlockHeight,
    epochs_in_interval: u32,
    epoch_duration_secs: u64,
) -> Result<Response, MixnetContractError> {
    // We don't have to check for authorization as this event can only be pushed
    // by the authorized entity.
    // Furthermore, we don't need to check whether the interval is finished as the
    // queue is only emptied upon the interval finishing.
    let interval = storage::current_interval(deps.storage)?;

    change_interval_config(
        deps.storage,
        created_at,
        interval,
        epochs_in_interval,
        epoch_duration_secs,
    )
}

impl ContractExecutableEvent for PendingIntervalEventData {
    fn execute(self, deps: DepsMut<'_>, _env: &Env) -> Result<Response, MixnetContractError> {
        // note that the basic validation on all those events was already performed before
        // they were pushed onto the queue
        match self.kind {
            PendingIntervalEventKind::ChangeMixCostParams { mix_id, new_costs } => {
                change_mix_cost_params(deps, self.created_at, mix_id, new_costs)
            }
            PendingIntervalEventKind::ChangeNymNodeCostParams { node_id, new_costs } => {
                change_nym_node_cost_params(deps, self.created_at, node_id, new_costs)
            }
            PendingIntervalEventKind::UpdateRewardingParams { update } => {
                update_rewarding_params(deps, self.created_at, update)
            }
            PendingIntervalEventKind::UpdateIntervalConfig {
                epochs_in_interval,
                epoch_duration_secs,
            } => update_interval_config(
                deps,
                self.created_at,
                epochs_in_interval,
                epoch_duration_secs,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::test_helpers::{assert_decimals, TestSetup};
    use cosmwasm_std::Decimal;
    use mixnet_contract_common::Percent;
    use std::time::Duration;

    // note that authorization and basic validation has already been performed for all of those
    // before being pushed onto the event queues

    #[cfg(test)]
    mod delegating {
        use super::*;
        use crate::mixnodes::transactions::try_remove_mixnode;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use crate::support::tests::test_helpers::get_bank_send_msg;
        use cosmwasm_std::coin;
        use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;

        #[test]
        fn returns_the_tokens_if_mixnode_has_unbonded() {
            let mut test = TestSetup::new();
            let mix_id = test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner"), None);

            let delegation = 120_000_000u128;
            let delegation_coin = coin(delegation, TEST_COIN_DENOM);
            let owner1 = &test.make_addr("delegator1");
            let owner2 = &test.make_addr("delegator2");

            // add pre-existing delegation
            test.add_immediate_delegation(owner1, delegation, mix_id);

            let env = test.env();
            unbond_mixnode(test.deps_mut(), &env, 123, mix_id).unwrap();

            let res_increase = delegate(
                test.deps_mut(),
                &env,
                123,
                Addr::unchecked(owner1),
                mix_id,
                delegation_coin.clone(),
            )
            .unwrap();

            // delegation wasn't increased
            let storage_key = Delegation::generate_storage_key(mix_id, owner1, None);
            let amount = delegations_storage::delegations()
                .load(test.deps().storage, storage_key)
                .unwrap()
                .amount;
            assert_eq!(amount, delegation_coin);

            // and all tokens are returned back to the delegator
            let (receiver, sent_amount) = get_bank_send_msg(&res_increase).unwrap();
            assert_eq!(receiver, owner1.to_string());
            assert_eq!(sent_amount[0], delegation_coin);

            // for a fresh delegation, nothing was added to the storage either
            let res_fresh = delegate(
                test.deps_mut(),
                &env,
                123,
                Addr::unchecked(owner2),
                mix_id,
                delegation_coin.clone(),
            )
            .unwrap();
            let storage_key =
                Delegation::generate_storage_key(mix_id, &Addr::unchecked(owner2), None);
            assert!(delegations_storage::delegations()
                .may_load(test.deps().storage, storage_key)
                .unwrap()
                .is_none());

            // and all tokens are returned back to the delegator
            let (receiver, sent_amount) = get_bank_send_msg(&res_fresh).unwrap();
            assert_eq!(receiver, owner2.to_string());
            assert_eq!(sent_amount[0], delegation_coin);
        }

        #[test]
        fn returns_the_tokens_is_mixnode_is_unbonding() {
            let mut test = TestSetup::new();
            let mix_id = test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner"), None);

            let delegation = 120_000_000u128;
            let delegation_coin = coin(delegation, TEST_COIN_DENOM);
            let owner1 = &test.make_addr("delegator1");
            let owner2 = &test.make_addr("delegator2");

            // add pre-existing delegation
            test.add_immediate_delegation(owner1, delegation, mix_id);

            let env = test.env();
            let sender = test.make_sender("mix-owner");
            try_remove_mixnode(test.deps_mut(), env.clone(), sender).unwrap();

            let res_increase = delegate(
                test.deps_mut(),
                &env,
                123,
                Addr::unchecked(owner1),
                mix_id,
                delegation_coin.clone(),
            )
            .unwrap();

            // delegation wasn't increased
            let storage_key = Delegation::generate_storage_key(mix_id, owner1, None);
            let amount = delegations_storage::delegations()
                .load(test.deps().storage, storage_key)
                .unwrap()
                .amount;
            assert_eq!(amount, delegation_coin);

            // and all tokens are returned back to the delegator
            let (receiver, sent_amount) = get_bank_send_msg(&res_increase).unwrap();
            assert_eq!(receiver, owner1.to_string());
            assert_eq!(sent_amount[0], delegation_coin);

            // for a fresh delegation, nothing was added to the storage either
            let res_fresh = delegate(
                test.deps_mut(),
                &env,
                123,
                Addr::unchecked(owner2),
                mix_id,
                delegation_coin.clone(),
            )
            .unwrap();
            let storage_key = Delegation::generate_storage_key(mix_id, owner2, None);
            assert!(delegations_storage::delegations()
                .may_load(test.deps().storage, storage_key)
                .unwrap()
                .is_none());

            // and all tokens are returned back to the delegator
            let (receiver, sent_amount) = get_bank_send_msg(&res_fresh).unwrap();
            assert_eq!(receiver, owner2.to_string());
            assert_eq!(sent_amount[0], delegation_coin);
        }

        #[test]
        fn if_delegation_already_exists_a_fresh_one_with_sum_of_both_is_created() {
            let mut test = TestSetup::new();
            let mix_id = test.add_rewarded_legacy_mixnode(
                &test.make_addr("mix-owner"),
                Some(100_000_000_000u128.into()),
            );

            let delegation_og = 120_000_000u128;
            let delegation_new = 543_000_000u128;
            let delegation_coin_new = coin(delegation_new, TEST_COIN_DENOM);

            let owner = &test.make_addr("delegator");
            test.add_immediate_delegation(owner, delegation_og, mix_id);

            let env = test.env();
            let res = delegate(
                test.deps_mut(),
                &env,
                123,
                Addr::unchecked(owner),
                mix_id,
                delegation_coin_new,
            )
            .unwrap();

            let expected_amount = delegation_og + delegation_new;
            let expected_amount_dec = Decimal::from_atomics(expected_amount, 0).unwrap();

            // no refunds here!
            assert!(get_bank_send_msg(&res).is_none());

            let rewarding = rewards_storage::MIXNODE_REWARDING
                .load(test.deps().storage, mix_id)
                .unwrap();
            let storage_key = Delegation::generate_storage_key(mix_id, owner, None);
            let delegation = delegations_storage::delegations()
                .load(test.deps().storage, storage_key)
                .unwrap();

            assert_eq!(rewarding.unique_delegations, 1);
            assert_eq!(rewarding.delegates, expected_amount_dec);

            assert_eq!(delegation.amount.amount.u128(), expected_amount)
        }

        #[test]
        fn if_delegation_already_exists_with_unclaimed_rewards_fresh_one_is_created() {
            let mut test = TestSetup::new();
            let mix_id = test.add_rewarded_legacy_mixnode(
                &test.make_addr("mix-owner"),
                Some(100_000_000_000u128.into()),
            );

            let delegation_og = 120_000_000u128;
            let delegation_new = 543_000_000u128;
            let delegation_coin_new = coin(delegation_new, TEST_COIN_DENOM);
            let active_params = test.active_node_params(100.0);

            // perform some rewarding here to advance the unit delegation beyond the initial value
            test.force_change_mix_rewarded_set(vec![mix_id]);
            test.skip_to_next_epoch_end();
            test.reward_with_distribution_ignore_state(mix_id, active_params);
            test.skip_to_next_epoch_end();
            test.reward_with_distribution_ignore_state(mix_id, active_params);
            test.skip_to_next_epoch_end();
            test.reward_with_distribution_ignore_state(mix_id, active_params);

            let owner = &test.make_addr("delegator");
            test.add_immediate_delegation(owner, delegation_og, mix_id);

            test.skip_to_next_epoch_end();
            let dist1 = test.reward_with_distribution_ignore_state(mix_id, active_params);
            test.skip_to_next_epoch_end();
            let dist2 = test.reward_with_distribution_ignore_state(mix_id, active_params);

            let storage_key = Delegation::generate_storage_key(mix_id, owner, None);
            let delegation_pre = delegations_storage::delegations()
                .load(test.deps().storage, storage_key.clone())
                .unwrap();

            let env = test.env();
            let res = delegate(
                test.deps_mut(),
                &env,
                123,
                Addr::unchecked(owner),
                mix_id,
                delegation_coin_new,
            )
            .unwrap();

            let earned_before_update = dist1.delegates + dist2.delegates;
            let truncated_reward = truncate_reward_amount(earned_before_update);

            let expected_amount = delegation_og + delegation_new + truncated_reward.u128();
            let expected_amount_dec = Decimal::from_atomics(expected_amount, 0).unwrap();

            // no refunds here!
            assert!(get_bank_send_msg(&res).is_none());

            let rewarding = test.mix_rewarding(mix_id);
            let delegation_post = delegations_storage::delegations()
                .load(test.deps().storage, storage_key)
                .unwrap();

            assert_ne!(
                delegation_pre.cumulative_reward_ratio,
                delegation_post.cumulative_reward_ratio
            );
            assert_eq!(
                delegation_post.cumulative_reward_ratio,
                rewarding.total_unit_reward
            );

            assert_eq!(rewarding.unique_delegations, 1);
            assert_eq!(rewarding.delegates, expected_amount_dec);

            assert_eq!(delegation_post.amount.amount.u128(), expected_amount)
        }

        #[test]
        fn appropriately_updates_state_for_fresh_delegation() {
            let mut test = TestSetup::new();
            let mix_id = test.add_rewarded_legacy_mixnode(
                &test.make_addr("mix-owner"),
                Some(100_000_000_000u128.into()),
            );
            let owner = "delegator";

            let delegation = 120_000_000u128;
            let delegation_coin = coin(120_000_000u128, TEST_COIN_DENOM);
            let active_params = test.active_node_params(100.0);

            // perform some rewarding here to advance the unit delegation beyond the initial value
            test.force_change_mix_rewarded_set(vec![mix_id]);
            test.skip_to_next_epoch_end();
            test.reward_with_distribution_ignore_state(mix_id, active_params);
            test.skip_to_next_epoch_end();
            test.reward_with_distribution_ignore_state(mix_id, active_params);

            let storage_key =
                Delegation::generate_storage_key(mix_id, &Addr::unchecked(owner), None);
            let delegation_pre = delegations_storage::delegations()
                .may_load(test.deps().storage, storage_key.clone())
                .unwrap();
            let rewarding_pre = test.mix_rewarding(mix_id);
            assert!(delegation_pre.is_none());
            assert!(rewarding_pre.delegates.is_zero());

            let env = test.env();
            let res = delegate(
                test.deps_mut(),
                &env,
                123,
                Addr::unchecked(owner),
                mix_id,
                delegation_coin.clone(),
            )
            .unwrap();
            assert!(get_bank_send_msg(&res).is_none());

            let delegation_post = delegations_storage::delegations()
                .load(test.deps().storage, storage_key)
                .unwrap();
            let rewarding_post = test.mix_rewarding(mix_id);
            assert_eq!(delegation_post.amount, delegation_coin);
            assert_eq!(
                delegation_post.cumulative_reward_ratio,
                rewarding_post.total_unit_reward
            );
            assert_eq!(
                rewarding_post.delegates,
                Decimal::from_atomics(delegation, 0).unwrap()
            )
        }
    }

    #[cfg(test)]
    mod undelegating {
        use super::*;
        use crate::support::tests::test_helpers::get_bank_send_msg;
        use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;

        #[test]
        fn doesnt_return_any_tokens_if_it_doesnt_exist() {
            let mut test = TestSetup::new();
            let mix_id = test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner"), None);

            let owner = Addr::unchecked("delegator");

            let res = undelegate(test.deps_mut(), 123, owner, mix_id).unwrap();
            assert!(get_bank_send_msg(&res).is_none());
        }

        #[test]
        fn errors_out_if_mix_rewarding_doesnt_exist() {
            let mut test = TestSetup::new();
            let mix_id = test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner"), None);

            let owner = &test.make_addr("delegator");
            test.add_immediate_delegation(owner, 100_000_000u32, mix_id);

            // this should never happen in actual code, but if we manually messed something up,
            // lets make sure this throws an error
            rewards_storage::MIXNODE_REWARDING.remove(test.deps_mut().storage, mix_id);
            let res = undelegate(test.deps_mut(), 123, owner.clone(), mix_id);
            assert!(matches!(
                res,
                Err(MixnetContractError::InconsistentState { .. })
            ));
        }

        #[test]
        fn returns_all_delegated_tokens_with_earned_rewards() {
            let mut test = TestSetup::new();
            let mix_id = test.add_rewarded_legacy_mixnode(
                &test.make_addr("mix-owner"),
                Some(100_000_000_000u128.into()),
            );

            let owner = &test.make_addr("delegator");
            let delegation = 120_000_000u128;

            let active_params = test.active_node_params(100.0);

            // perform some rewarding here to advance the unit delegation beyond the initial value
            test.force_change_mix_rewarded_set(vec![mix_id]);
            test.skip_to_next_epoch_end();
            test.reward_with_distribution_ignore_state(mix_id, active_params);
            test.skip_to_next_epoch_end();
            test.reward_with_distribution_ignore_state(mix_id, active_params);

            test.add_immediate_delegation(owner, delegation, mix_id);

            test.skip_to_next_epoch_end();
            let dist1 = test.reward_with_distribution_ignore_state(mix_id, active_params);
            test.skip_to_next_epoch_end();
            let dist2 = test.reward_with_distribution_ignore_state(mix_id, active_params);
            let expected_reward = dist1.delegates + dist2.delegates;
            let truncated_reward = truncate_reward_amount(expected_reward);

            let expected_return = delegation + truncated_reward.u128();

            let res = undelegate(test.deps_mut(), 123, Addr::unchecked(owner), mix_id).unwrap();
            let (receiver, sent_amount) = get_bank_send_msg(&res).unwrap();
            assert_eq!(receiver, owner.to_string());
            assert_eq!(sent_amount[0].amount.u128(), expected_return);

            // make sure delegation no longer exists
            let storage_key =
                Delegation::generate_storage_key(mix_id, &Addr::unchecked(owner), None);
            assert!(delegations_storage::delegations()
                .may_load(test.deps().storage, storage_key)
                .unwrap()
                .is_none());

            // and mix rewarding no longer contains any information about the delegation
            let rewarding = test.mix_rewarding(mix_id);
            assert!(rewarding.delegates.is_zero());
            assert_eq!(rewarding.unique_delegations, 0);
        }
    }

    #[cfg(test)]
    mod redelegating {
        use super::*;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use crate::support::tests::test_helpers::get_bank_send_msg;
        use cosmwasm_std::coin;
        use mixnet_contract_common::events::MixnetEventType;
        use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;

        fn target(to_node_id: NodeId, weight: u32) -> RedelegationTarget {
            RedelegationTarget { to_node_id, weight }
        }

        fn stored_delegation(
            test: &TestSetup,
            node_id: NodeId,
            owner: &Addr,
        ) -> Option<Delegation> {
            let key = Delegation::generate_storage_key(node_id, owner, None);
            delegations_storage::delegations()
                .may_load(test.deps().storage, key)
                .unwrap()
        }

        #[test]
        fn largest_remainder_preserves_the_total() {
            let targets = vec![target(10, 1), target(20, 2), target(30, 3)];
            let shares = split_by_weight(Uint128::new(10), &targets).unwrap();

            assert_eq!(
                shares,
                vec![Uint128::new(2), Uint128::new(3), Uint128::new(5)]
            );
            assert_eq!(shares.into_iter().sum::<Uint128>(), Uint128::new(10));
        }

        #[test]
        fn equal_remainders_are_tied_by_node_id_not_input_position() {
            let first_order = vec![target(20, 1), target(10, 1)];
            let second_order = vec![target(10, 1), target(20, 1)];

            let first = split_by_weight(Uint128::one(), &first_order).unwrap();
            let second = split_by_weight(Uint128::one(), &second_order).unwrap();

            assert_eq!(first, vec![Uint128::zero(), Uint128::one()]);
            assert_eq!(second, vec![Uint128::one(), Uint128::zero()]);
        }

        #[test]
        fn split_rejects_invalid_inputs_and_handles_maximum_values() {
            assert!(split_by_weight(Uint128::one(), &[]).is_err());
            assert!(split_by_weight(Uint128::one(), &[target(10, 0)]).is_err());

            let targets = (0..MAX_REDELEGATION_TARGETS)
                .map(|i| target(i as NodeId + 10, u32::MAX))
                .collect::<Vec<_>>();
            let shares = split_by_weight(Uint128::MAX, &targets).unwrap();
            assert_eq!(shares.into_iter().sum::<Uint128>(), Uint128::MAX);
        }

        #[test]
        fn shape_validation_rejects_invalid_payloads() {
            assert!(validate_redelegation_shape(&[], 1).is_err());
            assert!(validate_redelegation_shape(&[target(1, 1)], 1).is_err());
            assert!(validate_redelegation_shape(&[target(2, 0)], 1).is_err());
            assert!(validate_redelegation_shape(&[target(2, 1), target(2, 2)], 1).is_err());

            let too_many = (0..=MAX_REDELEGATION_TARGETS)
                .map(|i| target(i as NodeId + 2, 1))
                .collect::<Vec<_>>();
            assert!(validate_redelegation_shape(&too_many, 1).is_err());
        }

        #[test]
        fn moves_the_entire_delegation_without_a_bank_send() {
            let mut test = TestSetup::new();
            let source = test.add_rewarded_legacy_mixnode(&test.make_addr("source-owner"), None);
            let destination =
                test.add_rewarded_legacy_mixnode(&test.make_addr("destination-owner"), None);
            let owner = test.make_addr("delegator");
            let amount = 120_000_000u128;
            test.add_immediate_delegation(&owner, amount, source);

            let env = test.env();
            let response = redelegate(
                test.deps_mut(),
                &env,
                123,
                owner.clone(),
                source,
                vec![target(destination, 1)],
            )
            .unwrap();

            assert!(get_bank_send_msg(&response).is_none());
            assert!(stored_delegation(&test, source, &owner).is_none());
            assert_eq!(
                stored_delegation(&test, destination, &owner)
                    .unwrap()
                    .amount
                    .amount,
                Uint128::new(amount)
            );
            assert_eq!(test.mix_rewarding(source).unique_delegations, 0);
            assert_eq!(test.mix_rewarding(destination).unique_delegations, 1);
        }

        #[test]
        fn missing_source_is_reported_without_touching_the_target() {
            let mut test = TestSetup::new();
            let source = test.add_rewarded_legacy_mixnode(&test.make_addr("source-owner"), None);
            let destination =
                test.add_rewarded_legacy_mixnode(&test.make_addr("destination-owner"), None);
            let owner = test.make_addr("delegator");
            let env = test.env();

            let response = redelegate(
                test.deps_mut(),
                &env,
                123,
                owner.clone(),
                source,
                vec![target(destination, 1)],
            )
            .unwrap();

            assert!(get_bank_send_msg(&response).is_none());
            assert!(stored_delegation(&test, destination, &owner).is_none());
            assert_eq!(
                response.events[0].ty,
                MixnetEventType::RedelegationFailed.to_string()
            );
            assert!(response.events[0]
                .attributes
                .iter()
                .any(|attr| attr.key == "error_message"
                    && attr.value == "source delegation no longer exists"));
        }

        #[test]
        fn splits_the_settled_amount_across_targets() {
            let mut test = TestSetup::new();
            let source = test.add_rewarded_legacy_mixnode(&test.make_addr("source-owner"), None);
            let first = test.add_rewarded_legacy_mixnode(&test.make_addr("first-owner"), None);
            let second = test.add_rewarded_legacy_mixnode(&test.make_addr("second-owner"), None);
            let owner = test.make_addr("delegator");
            let amount = 120_000_000u128;
            test.add_immediate_delegation(&owner, amount, source);

            let env = test.env();
            let response = redelegate(
                test.deps_mut(),
                &env,
                123,
                owner.clone(),
                source,
                vec![target(first, 1), target(second, 3)],
            )
            .unwrap();

            assert!(get_bank_send_msg(&response).is_none());
            assert!(stored_delegation(&test, source, &owner).is_none());
            assert_eq!(
                stored_delegation(&test, first, &owner)
                    .unwrap()
                    .amount
                    .amount,
                Uint128::new(30_000_000)
            );
            assert_eq!(
                stored_delegation(&test, second, &owner)
                    .unwrap()
                    .amount
                    .amount,
                Uint128::new(90_000_000)
            );
        }

        #[test]
        fn unavailable_target_leaves_the_source_unchanged() {
            let mut test = TestSetup::new();
            let source = test.add_rewarded_legacy_mixnode(&test.make_addr("source-owner"), None);
            let available =
                test.add_rewarded_legacy_mixnode(&test.make_addr("available-owner"), None);
            let destination =
                test.add_rewarded_legacy_mixnode(&test.make_addr("destination-owner"), None);
            let owner = test.make_addr("delegator");
            let amount = 120_000_000u128;
            test.add_immediate_delegation(&owner, amount, source);
            let source_rewarding = test.mix_rewarding(source);

            let env = test.env();
            unbond_mixnode(test.deps_mut(), &env, 123, destination).unwrap();
            let response = redelegate(
                test.deps_mut(),
                &env,
                124,
                owner.clone(),
                source,
                vec![target(available, 1), target(destination, 1)],
            )
            .unwrap();

            assert!(get_bank_send_msg(&response).is_none());
            assert_eq!(
                stored_delegation(&test, source, &owner)
                    .unwrap()
                    .amount
                    .amount,
                Uint128::new(amount)
            );
            assert_eq!(test.mix_rewarding(source), source_rewarding);
            assert!(stored_delegation(&test, available, &owner).is_none());
            assert!(stored_delegation(&test, destination, &owner).is_none());
        }

        #[test]
        fn share_below_minimum_leaves_the_source_unchanged() {
            let mut test = TestSetup::new();
            let source = test.add_rewarded_legacy_mixnode(&test.make_addr("source-owner"), None);
            let first = test.add_rewarded_legacy_mixnode(&test.make_addr("first-owner"), None);
            let second = test.add_rewarded_legacy_mixnode(&test.make_addr("second-owner"), None);
            let owner = test.make_addr("delegator");
            let amount = 200u128;
            test.add_immediate_delegation(&owner, amount, source);

            let mut state = mixnet_params_storage::CONTRACT_STATE
                .load(test.deps().storage)
                .unwrap();
            state.params.delegations_params.minimum_delegation = Some(coin(150, TEST_COIN_DENOM));
            mixnet_params_storage::CONTRACT_STATE
                .save(test.deps_mut().storage, &state)
                .unwrap();

            let env = test.env();
            let response = redelegate(
                test.deps_mut(),
                &env,
                123,
                owner.clone(),
                source,
                vec![target(first, 1), target(second, 1)],
            )
            .unwrap();

            assert!(get_bank_send_msg(&response).is_none());
            assert_eq!(
                stored_delegation(&test, source, &owner)
                    .unwrap()
                    .amount
                    .amount,
                Uint128::new(amount)
            );
            assert!(stored_delegation(&test, first, &owner).is_none());
            assert!(stored_delegation(&test, second, &owner).is_none());
        }

        #[test]
        fn zero_share_is_rejected_even_when_the_configured_minimum_is_zero() {
            let mut test = TestSetup::new();
            let source = test.add_rewarded_legacy_mixnode(&test.make_addr("source-owner"), None);
            let first = test.add_rewarded_legacy_mixnode(&test.make_addr("first-owner"), None);
            let second = test.add_rewarded_legacy_mixnode(&test.make_addr("second-owner"), None);
            let owner = test.make_addr("delegator");
            test.add_immediate_delegation(&owner, 1u128, source);

            let mut state = mixnet_params_storage::CONTRACT_STATE
                .load(test.deps().storage)
                .unwrap();
            state.params.delegations_params.minimum_delegation = Some(coin(0, TEST_COIN_DENOM));
            mixnet_params_storage::CONTRACT_STATE
                .save(test.deps_mut().storage, &state)
                .unwrap();

            let env = test.env();
            let response = redelegate(
                test.deps_mut(),
                &env,
                123,
                owner.clone(),
                source,
                vec![target(first, 1), target(second, 1)],
            )
            .unwrap();

            assert!(get_bank_send_msg(&response).is_none());
            assert_eq!(
                stored_delegation(&test, source, &owner)
                    .unwrap()
                    .amount
                    .amount,
                Uint128::one()
            );
            assert!(stored_delegation(&test, first, &owner).is_none());
            assert!(stored_delegation(&test, second, &owner).is_none());
        }

        #[test]
        fn moves_accrued_reward_with_the_source_stake() {
            let mut test = TestSetup::new();
            let source = test.add_rewarded_legacy_mixnode(
                &test.make_addr("source-owner"),
                Some(100_000_000_000u128.into()),
            );
            let destination =
                test.add_rewarded_legacy_mixnode(&test.make_addr("destination-owner"), None);
            let owner = test.make_addr("delegator");
            let amount = 120_000_000u128;
            test.add_immediate_delegation(&owner, amount, source);

            let active_params = test.active_node_params(100.0);
            test.force_change_mix_rewarded_set(vec![source]);
            test.skip_to_next_epoch_end();
            let distribution = test.reward_with_distribution_ignore_state(source, active_params);
            let expected = amount + truncate_reward_amount(distribution.delegates).u128();

            let env = test.env();
            redelegate(
                test.deps_mut(),
                &env,
                123,
                owner.clone(),
                source,
                vec![target(destination, 1)],
            )
            .unwrap();

            assert!(stored_delegation(&test, source, &owner).is_none());
            assert_eq!(
                stored_delegation(&test, destination, &owner)
                    .unwrap()
                    .amount
                    .amount,
                Uint128::new(expected)
            );
        }

        #[test]
        fn compounds_into_an_existing_target_delegation() {
            let mut test = TestSetup::new();
            let source = test.add_rewarded_legacy_mixnode(&test.make_addr("source-owner"), None);
            let destination =
                test.add_rewarded_legacy_mixnode(&test.make_addr("destination-owner"), None);
            let owner = test.make_addr("delegator");
            test.add_immediate_delegation(&owner, 120_000_000u128, source);
            test.add_immediate_delegation(&owner, 80_000_000u128, destination);

            let env = test.env();
            redelegate(
                test.deps_mut(),
                &env,
                123,
                owner.clone(),
                source,
                vec![target(destination, 1)],
            )
            .unwrap();

            assert!(stored_delegation(&test, source, &owner).is_none());
            assert_eq!(
                stored_delegation(&test, destination, &owner)
                    .unwrap()
                    .amount
                    .amount,
                Uint128::new(200_000_000)
            );
            assert_eq!(test.mix_rewarding(destination).unique_delegations, 1);
        }
    }

    #[cfg(test)]
    mod mixnode_unbonding {
        use crate::compat::transactions::{try_decrease_pledge, try_increase_pledge};
        use crate::interval::pending_events::unbond_mixnode;
        use crate::mixnodes::storage as mixnodes_storage;
        use crate::rewards::storage as rewards_storage;
        use crate::support::tests::test_helpers::{get_bank_send_msg, TestSetup};
        use cosmwasm_std::testing::message_info;
        use cosmwasm_std::{Addr, Uint128};
        use mixnet_contract_common::error::MixnetContractError;
        use mixnet_contract_common::mixnode::{PendingMixNodeChanges, UnbondedMixnode};
        use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;

        #[test]
        fn returns_hard_error_if_mixnode_doesnt_exist() {
            // this should have never happened so hard error MUST be thrown here
            let mut test = TestSetup::new();
            let env = test.env();

            let res = unbond_mixnode(test.deps_mut(), &env, 123, 1);
            assert!(matches!(
                res,
                Err(MixnetContractError::InconsistentState { .. })
            ));
        }

        #[test]
        fn returns_hard_error_if_there_are_pending_pledge_changes() {
            let mut test = TestSetup::new();
            let env = test.env();
            let change = test.coins(1234);

            // increase
            let owner = &test.make_addr("mix-owner1");
            let pledge = Uint128::new(250_000_000);
            let mix_id = test.add_rewarded_legacy_mixnode(owner, Some(pledge));

            try_increase_pledge(test.deps_mut(), env.clone(), message_info(owner, &change))
                .unwrap();

            let res = unbond_mixnode(test.deps_mut(), &env, 123, mix_id);
            assert!(matches!(
                res,
                Err(MixnetContractError::InconsistentState { .. })
            ));

            // decrease
            let owner = &test.make_addr("mix-owner2");
            let pledge = Uint128::new(250_000_000);
            let mix_id = test.add_rewarded_legacy_mixnode(owner, Some(pledge));

            try_decrease_pledge(
                test.deps_mut(),
                env.clone(),
                message_info(owner, &[]),
                change[0].clone(),
            )
            .unwrap();

            let res = unbond_mixnode(test.deps_mut(), &env, 123, mix_id);
            assert!(matches!(
                res,
                Err(MixnetContractError::InconsistentState { .. })
            ));

            // artificial
            let owner = &test.make_addr("mix-owner3");
            let pledge = Uint128::new(250_000_000);
            let mix_id = test.add_rewarded_legacy_mixnode(owner, Some(pledge));

            let changes = PendingMixNodeChanges {
                pledge_change: Some(1234),
                cost_params_change: None,
            };

            mixnodes_storage::PENDING_MIXNODE_CHANGES
                .save(test.deps_mut().storage, mix_id, &changes)
                .unwrap();
            let res = unbond_mixnode(test.deps_mut(), &env, 123, mix_id);
            assert!(matches!(
                res,
                Err(MixnetContractError::InconsistentState { .. })
            ));
        }

        #[test]
        fn returns_original_pledge_alongside_any_earned_rewards() {
            let mut test = TestSetup::new();

            let owner = &test.make_addr("mix-owner");
            let pledge = Uint128::new(250_000_000);
            let mix_id = test.add_rewarded_legacy_mixnode(owner, Some(pledge));
            let mix_details = mixnodes_storage::mixnode_bonds()
                .load(test.deps().storage, mix_id)
                .unwrap();
            let active_params = test.active_node_params(100.0);

            test.force_change_mix_rewarded_set(vec![mix_id]);
            test.skip_to_next_epoch_end();
            let dist1 = test.reward_with_distribution_ignore_state(mix_id, active_params);
            test.skip_to_next_epoch_end();
            let dist2 = test.reward_with_distribution_ignore_state(mix_id, active_params);

            let expected_reward = dist1.operator + dist2.operator;
            let truncated_reward = truncate_reward_amount(expected_reward);
            let expected_return = pledge + truncated_reward;

            let env = test.env();
            let res = unbond_mixnode(test.deps_mut(), &env, 123, mix_id).unwrap();
            let (receiver, sent_amount) = get_bank_send_msg(&res).unwrap();
            assert_eq!(receiver, owner.to_string());
            assert_eq!(sent_amount[0].amount, expected_return);

            assert!(rewards_storage::MIXNODE_REWARDING
                .may_load(test.deps().storage, mix_id)
                .unwrap()
                .is_none());
            assert!(mixnodes_storage::mixnode_bonds()
                .may_load(test.deps().storage, mix_id)
                .unwrap()
                .is_none());
            let expected = UnbondedMixnode {
                identity_key: mix_details.identity().to_string(),
                owner: Addr::unchecked(owner),
                proxy: None,
                unbonding_height: env.block.height,
            };
            assert_eq!(
                expected,
                mixnodes_storage::unbonded_mixnodes()
                    .load(test.deps().storage, mix_id)
                    .unwrap()
            );
        }
    }

    #[cfg(test)]
    mod increasing_pledge {
        use cosmwasm_std::Uint128;

        use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;

        use super::*;

        #[test]
        fn returns_hard_error_if_mixnode_doesnt_exist() {
            // this should have never happened so hard error MUST be thrown here
            let mut test = TestSetup::new();

            let amount = test.coin(123);
            let res = increase_mixnode_pledge(test.deps_mut(), 123, 1, amount);
            assert!(matches!(
                res,
                Err(MixnetContractError::InconsistentState { .. })
            ));
        }

        #[test]
        fn returns_hard_error_if_there_are_no_pending_pledge_changes() {
            let mut test = TestSetup::new();
            let change = test.coin(1234);

            let owner = &test.make_addr("mix-owner");
            let pledge = Uint128::new(250_000_000);
            let mix_id = test.add_rewarded_legacy_mixnode(owner, Some(pledge));

            let res = increase_mixnode_pledge(test.deps_mut(), 123, mix_id, change);
            assert!(matches!(
                res,
                Err(MixnetContractError::InconsistentState { .. })
            ));
        }

        #[test]
        fn updates_stored_bond_information_and_rewarding_details() {
            let mut test = TestSetup::new();
            let mix_id = test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner"), None);
            test.set_pending_pledge_change(mix_id, None);

            let old_details = get_mixnode_details_by_id(test.deps().storage, mix_id)
                .unwrap()
                .unwrap();

            let amount = test.coin(12345);
            increase_mixnode_pledge(test.deps_mut(), 123, mix_id, amount.clone()).unwrap();

            let updated_details = get_mixnode_details_by_id(test.deps().storage, mix_id)
                .unwrap()
                .unwrap();

            assert_eq!(
                updated_details.bond_information.original_pledge.amount,
                old_details.bond_information.original_pledge.amount + amount.amount
            );

            assert_eq!(
                updated_details.rewarding_details.operator,
                old_details.rewarding_details.operator
                    + Decimal::from_atomics(amount.amount, 0).unwrap()
            );
        }

        #[test]
        fn without_any_events_in_between_is_equivalent_to_pledging_the_same_amount_immediately() {
            let mut test = TestSetup::new();
            let pledge1 = Uint128::new(150_000_000);
            let pledge2 = Uint128::new(50_000_000);
            let pledge3 = Uint128::new(200_000_000);
            let active_params = test.active_node_params(100.0);

            let mix_id_repledge =
                test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner1"), Some(pledge1));
            test.set_pending_pledge_change(mix_id_repledge, None);

            let increase = test.coin(pledge2.u128());
            increase_mixnode_pledge(test.deps_mut(), 123, mix_id_repledge, increase).unwrap();

            let mix_id_full_pledge =
                test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner2"), Some(pledge3));

            test.add_immediate_delegation(
                &test.make_addr("alice"),
                123_456_789u128,
                mix_id_repledge,
            );
            test.add_immediate_delegation(&test.make_addr("bob"), 500_000_000u128, mix_id_repledge);
            test.add_immediate_delegation(
                &test.make_addr("carol"),
                111_111_111u128,
                mix_id_repledge,
            );

            test.add_immediate_delegation(
                &test.make_addr("alice"),
                123_456_789u128,
                mix_id_full_pledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("bob"),
                500_000_000u128,
                mix_id_full_pledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("carol"),
                111_111_111u128,
                mix_id_full_pledge,
            );

            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![mix_id_repledge, mix_id_full_pledge]);

            let dist1 = test.reward_with_distribution_ignore_state(mix_id_repledge, active_params);
            let dist2 =
                test.reward_with_distribution_ignore_state(mix_id_full_pledge, active_params);

            assert_eq!(dist1, dist2)
        }

        #[test]
        fn correctly_increases_future_rewards() {
            let mut test = TestSetup::new();
            let pledge1 = Uint128::new(150_000_000_000);
            let pledge2 = Uint128::new(50_000_000_000);
            let active_params = test.active_node_params(100.0);

            let mix_id_repledge =
                test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner1"), Some(pledge1));
            test.set_pending_pledge_change(mix_id_repledge, None);

            test.add_immediate_delegation(
                &test.make_addr("alice"),
                123_456_789_000u128,
                mix_id_repledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("bob"),
                500_000_000_000u128,
                mix_id_repledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("carol"),
                111_111_111_000u128,
                mix_id_repledge,
            );

            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![mix_id_repledge]);

            let dist = test.reward_with_distribution_ignore_state(mix_id_repledge, active_params);

            let increase = test.coin(pledge2.u128());
            increase_mixnode_pledge(test.deps_mut(), 123, mix_id_repledge, increase).unwrap();

            let pledge3 = Uint128::new(200_000_000_000) + truncate_reward_amount(dist.operator);
            let mix_id_full_pledge =
                test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner2"), Some(pledge3));

            test.add_immediate_delegation(
                &test.make_addr("alice"),
                123_456_789_000u128,
                mix_id_full_pledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("bob"),
                500_000_000_000u128,
                mix_id_full_pledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("carol"),
                111_111_111_000u128,
                mix_id_full_pledge,
            );

            let lost_operator = dist.operator
                - Decimal::from_atomics(truncate_reward_amount(dist.operator), 0).unwrap();
            let lost_delegates = dist.delegates
                - Decimal::from_atomics(truncate_reward_amount(dist.delegates), 0).unwrap();

            // add the tiny bit of lost precision manually
            let mut mix_rewarding_full = test.mix_rewarding(mix_id_full_pledge);
            mix_rewarding_full.delegates += lost_delegates;
            mix_rewarding_full.operator += lost_operator;
            rewards_storage::MIXNODE_REWARDING
                .save(
                    test.deps_mut().storage,
                    mix_id_full_pledge,
                    &mix_rewarding_full,
                )
                .unwrap();

            test.add_immediate_delegation(
                &test.make_addr("dave"),
                truncate_reward_amount(dist.delegates).u128(),
                mix_id_full_pledge,
            );

            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![mix_id_repledge, mix_id_full_pledge]);

            // go through few epochs of rewarding
            for _ in 0..500 {
                test.skip_to_next_epoch_end();
                let dist1 =
                    test.reward_with_distribution_ignore_state(mix_id_repledge, active_params);
                let dist2 =
                    test.reward_with_distribution_ignore_state(mix_id_full_pledge, active_params);

                assert_eq!(dist1, dist2)
            }
        }

        #[test]
        fn correctly_increases_future_rewards_with_more_passed_epochs() {
            let mut test = TestSetup::new();
            let pledge1 = Uint128::new(150_000_000_000);
            let pledge2 = Uint128::new(50_000_000_000);
            let active_params = test.active_node_params(100.0);

            let mix_id_repledge =
                test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner1"), Some(pledge1));
            test.set_pending_pledge_change(mix_id_repledge, None);

            test.add_immediate_delegation(
                &test.make_addr("alice"),
                123_456_789_000u128,
                mix_id_repledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("bob"),
                500_000_000_000u128,
                mix_id_repledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("carol"),
                111_111_111_000u128,
                mix_id_repledge,
            );

            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![mix_id_repledge]);

            let mut cumulative_op_reward = Decimal::zero();
            let mut cumulative_del_reward = Decimal::zero();

            // go few epochs of rewarding before adding more pledge
            for _ in 0..500 {
                test.skip_to_next_epoch_end();
                let dist =
                    test.reward_with_distribution_ignore_state(mix_id_repledge, active_params);
                cumulative_op_reward += dist.operator;
                cumulative_del_reward += dist.delegates;
            }

            let increase = test.coin(pledge2.u128());
            increase_mixnode_pledge(test.deps_mut(), 123, mix_id_repledge, increase).unwrap();

            let pledge3 =
                Uint128::new(200_000_000_000) + truncate_reward_amount(cumulative_op_reward);
            let mix_id_full_pledge =
                test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner2"), Some(pledge3));

            test.add_immediate_delegation(
                &test.make_addr("alice"),
                123_456_789_000u128,
                mix_id_full_pledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("bob"),
                500_000_000_000u128,
                mix_id_full_pledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("carol"),
                111_111_111_000u128,
                mix_id_full_pledge,
            );

            let lost_operator = cumulative_op_reward
                - Decimal::from_atomics(truncate_reward_amount(cumulative_op_reward), 0).unwrap();
            let lost_delegates = cumulative_del_reward
                - Decimal::from_atomics(truncate_reward_amount(cumulative_del_reward), 0).unwrap();

            // add the tiny bit of lost precision manually
            let mut mix_rewarding_full = test.mix_rewarding(mix_id_full_pledge);
            mix_rewarding_full.delegates += lost_delegates;
            mix_rewarding_full.operator += lost_operator;
            rewards_storage::MIXNODE_REWARDING
                .save(
                    test.deps_mut().storage,
                    mix_id_full_pledge,
                    &mix_rewarding_full,
                )
                .unwrap();

            test.add_immediate_delegation(
                &test.make_addr("dave"),
                truncate_reward_amount(cumulative_del_reward).u128(),
                mix_id_full_pledge,
            );

            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![mix_id_repledge, mix_id_full_pledge]);

            // go through few more epochs of rewarding
            for _ in 0..500 {
                test.skip_to_next_epoch_end();
                let dist1 =
                    test.reward_with_distribution_ignore_state(mix_id_repledge, active_params);
                let dist2 =
                    test.reward_with_distribution_ignore_state(mix_id_full_pledge, active_params);

                assert_eq!(dist1, dist2)
            }
        }

        #[test]
        fn updates_the_pending_pledge_changes_field() {
            let mut test = TestSetup::new();
            let mix_id = test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner"), None);
            test.set_pending_pledge_change(mix_id, None);

            let amount = test.coin(12345);
            increase_mixnode_pledge(test.deps_mut(), 123, mix_id, amount).unwrap();
            let pending = mixnodes_storage::PENDING_MIXNODE_CHANGES
                .load(test.deps().storage, mix_id)
                .unwrap();
            assert!(pending.pledge_change.is_none())
        }
    }

    #[cfg(test)]
    mod decreasing_pledge {
        use super::*;
        use cosmwasm_std::{BankMsg, CosmosMsg, Uint128};
        use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;

        #[test]
        fn returns_hard_error_if_mixnode_doesnt_exist() {
            // this should have never happened so hard error MUST be thrown here
            let mut test = TestSetup::new();

            let amount = test.coin(123);
            let res = decrease_mixnode_pledge(test.deps_mut(), 123, 1, amount);
            assert!(matches!(
                res,
                Err(MixnetContractError::InconsistentState { .. })
            ));
        }

        #[test]
        fn returns_hard_error_if_there_are_no_pending_pledge_changes() {
            let mut test = TestSetup::new();
            let change = test.coin(1234);

            let owner = &test.make_addr("mix-owner");
            let pledge = Uint128::new(250_000_000);
            let mix_id = test.add_rewarded_legacy_mixnode(owner, Some(pledge));

            let res = decrease_mixnode_pledge(test.deps_mut(), 123, mix_id, change);
            assert!(matches!(
                res,
                Err(MixnetContractError::InconsistentState { .. })
            ));
        }

        #[test]
        fn updates_stored_bond_information_and_rewarding_details() {
            let mut test = TestSetup::new();
            let mix_id = test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner"), None);
            test.set_pending_pledge_change(mix_id, None);

            let old_details = get_mixnode_details_by_id(test.deps().storage, mix_id)
                .unwrap()
                .unwrap();

            let amount = test.coin(12345);
            decrease_mixnode_pledge(test.deps_mut(), 123, mix_id, amount.clone()).unwrap();

            let updated_details = get_mixnode_details_by_id(test.deps().storage, mix_id)
                .unwrap()
                .unwrap();

            assert_eq!(
                updated_details.bond_information.original_pledge.amount,
                old_details.bond_information.original_pledge.amount - amount.amount
            );

            assert_eq!(
                updated_details.rewarding_details.operator,
                old_details.rewarding_details.operator
                    - Decimal::from_atomics(amount.amount, 0).unwrap()
            );
        }

        #[test]
        fn returns_tokens_back_to_the_owner() {
            let mut test = TestSetup::new();
            let owner = &test.make_addr("mix-owner");
            let mix_id = test.add_rewarded_legacy_mixnode(owner, None);
            test.set_pending_pledge_change(mix_id, None);

            let amount = test.coin(12345);
            let res =
                decrease_mixnode_pledge(test.deps_mut(), 123, mix_id, amount.clone()).unwrap();

            assert_eq!(res.messages.len(), 1);
            assert_eq!(
                res.messages[0].msg,
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: owner.to_string(),
                    amount: vec![amount],
                })
            )
        }

        #[test]
        fn without_any_events_in_between_is_equivalent_to_pledging_the_same_amount_immediately() {
            let mut test = TestSetup::new();
            let pledge1 = Uint128::new(200_000_000);
            let pledge_change = Uint128::new(50_000_000);
            let pledge3 = Uint128::new(150_000_000);
            let active_params = test.active_node_params(100.0);

            let mix_id_repledge =
                test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner1"), Some(pledge1));
            test.set_pending_pledge_change(mix_id_repledge, None);

            let decrease = test.coin(pledge_change.u128());
            decrease_mixnode_pledge(test.deps_mut(), 123, mix_id_repledge, decrease).unwrap();

            let mix_id_full_pledge =
                test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner2"), Some(pledge3));

            test.add_immediate_delegation(
                &test.make_addr("alice"),
                123_456_789u128,
                mix_id_repledge,
            );
            test.add_immediate_delegation(&test.make_addr("bob"), 500_000_000u128, mix_id_repledge);
            test.add_immediate_delegation(
                &test.make_addr("carol"),
                111_111_111u128,
                mix_id_repledge,
            );

            test.add_immediate_delegation(
                &test.make_addr("alice"),
                123_456_789u128,
                mix_id_full_pledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("bob"),
                500_000_000u128,
                mix_id_full_pledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("carol"),
                111_111_111u128,
                mix_id_full_pledge,
            );

            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![mix_id_repledge, mix_id_full_pledge]);

            let dist1 = test.reward_with_distribution_ignore_state(mix_id_repledge, active_params);
            let dist2 =
                test.reward_with_distribution_ignore_state(mix_id_full_pledge, active_params);

            assert_eq!(dist1, dist2)
        }

        #[test]
        fn correctly_decreases_future_rewards() {
            let mut test = TestSetup::new();
            let pledge1 = Uint128::new(200_000_000_000);
            let pledge_change = Uint128::new(50_000_000_000);
            let active_params = test.active_node_params(100.0);

            let mix_id_repledge =
                test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner1"), Some(pledge1));
            test.set_pending_pledge_change(mix_id_repledge, None);

            test.add_immediate_delegation(
                &test.make_addr("alice"),
                123_456_789_000u128,
                mix_id_repledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("bob"),
                500_000_000_000u128,
                mix_id_repledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("carol"),
                111_111_111_000u128,
                mix_id_repledge,
            );

            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![mix_id_repledge]);

            let dist = test.reward_with_distribution_ignore_state(mix_id_repledge, active_params);

            let decrease = test.coin(pledge_change.u128());
            decrease_mixnode_pledge(test.deps_mut(), 123, mix_id_repledge, decrease).unwrap();

            let pledge3 = Uint128::new(150_000_000_000) + truncate_reward_amount(dist.operator);
            let mix_id_full_pledge =
                test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner2"), Some(pledge3));

            test.add_immediate_delegation(
                &test.make_addr("alice"),
                123_456_789_000u128,
                mix_id_full_pledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("bob"),
                500_000_000_000u128,
                mix_id_full_pledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("carol"),
                111_111_111_000u128,
                mix_id_full_pledge,
            );

            let lost_operator = dist.operator
                - Decimal::from_atomics(truncate_reward_amount(dist.operator), 0).unwrap();
            let lost_delegates = dist.delegates
                - Decimal::from_atomics(truncate_reward_amount(dist.delegates), 0).unwrap();

            // add the tiny bit of lost precision manually
            let mut mix_rewarding_full = test.mix_rewarding(mix_id_full_pledge);
            mix_rewarding_full.delegates += lost_delegates;
            mix_rewarding_full.operator += lost_operator;
            rewards_storage::MIXNODE_REWARDING
                .save(
                    test.deps_mut().storage,
                    mix_id_full_pledge,
                    &mix_rewarding_full,
                )
                .unwrap();

            test.add_immediate_delegation(
                &test.make_addr("dave"),
                truncate_reward_amount(dist.delegates).u128(),
                mix_id_full_pledge,
            );

            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![mix_id_repledge, mix_id_full_pledge]);

            // go through few epochs of rewarding
            for _ in 0..500 {
                test.skip_to_next_epoch_end();
                let dist1 =
                    test.reward_with_distribution_ignore_state(mix_id_repledge, active_params);
                let dist2 =
                    test.reward_with_distribution_ignore_state(mix_id_full_pledge, active_params);

                assert_eq!(dist1, dist2)
            }
        }

        #[test]
        fn correctly_decreases_future_rewards_with_more_passed_epochs() {
            let mut test = TestSetup::new();
            let pledge1 = Uint128::new(200_000_000_000);
            let pledge_change = Uint128::new(50_000_000_000);
            let active_params = test.active_node_params(100.0);

            let mix_id_repledge =
                test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner1"), Some(pledge1));
            test.set_pending_pledge_change(mix_id_repledge, None);

            test.add_immediate_delegation(
                &test.make_addr("alice"),
                123_456_789_000u128,
                mix_id_repledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("bob"),
                500_000_000_000u128,
                mix_id_repledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("carol"),
                111_111_111_000u128,
                mix_id_repledge,
            );

            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![mix_id_repledge]);

            let mut cumulative_op_reward = Decimal::zero();
            let mut cumulative_del_reward = Decimal::zero();

            // go few epochs of rewarding before decreasing pledge
            for _ in 0..500 {
                test.skip_to_next_epoch_end();
                let dist =
                    test.reward_with_distribution_ignore_state(mix_id_repledge, active_params);
                cumulative_op_reward += dist.operator;
                cumulative_del_reward += dist.delegates;
            }

            let decrease = test.coin(pledge_change.u128());
            decrease_mixnode_pledge(test.deps_mut(), 123, mix_id_repledge, decrease).unwrap();

            let pledge3 =
                Uint128::new(150_000_000_000) + truncate_reward_amount(cumulative_op_reward);
            let mix_id_full_pledge =
                test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner2"), Some(pledge3));

            test.add_immediate_delegation(
                &test.make_addr("alice"),
                123_456_789_000u128,
                mix_id_full_pledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("bob"),
                500_000_000_000u128,
                mix_id_full_pledge,
            );
            test.add_immediate_delegation(
                &test.make_addr("carol"),
                111_111_111_000u128,
                mix_id_full_pledge,
            );

            let lost_operator = cumulative_op_reward
                - Decimal::from_atomics(truncate_reward_amount(cumulative_op_reward), 0).unwrap();
            let lost_delegates = cumulative_del_reward
                - Decimal::from_atomics(truncate_reward_amount(cumulative_del_reward), 0).unwrap();

            // add the tiny bit of lost precision manually
            let mut mix_rewarding_full = test.mix_rewarding(mix_id_full_pledge);
            mix_rewarding_full.delegates += lost_delegates;
            mix_rewarding_full.operator += lost_operator;
            rewards_storage::MIXNODE_REWARDING
                .save(
                    test.deps_mut().storage,
                    mix_id_full_pledge,
                    &mix_rewarding_full,
                )
                .unwrap();

            test.add_immediate_delegation(
                &test.make_addr("dave"),
                truncate_reward_amount(cumulative_del_reward).u128(),
                mix_id_full_pledge,
            );

            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![mix_id_repledge, mix_id_full_pledge]);

            // go through few more epochs of rewarding
            for _ in 0..500 {
                test.skip_to_next_epoch_end();
                let dist1 =
                    test.reward_with_distribution_ignore_state(mix_id_repledge, active_params);
                let dist2 =
                    test.reward_with_distribution_ignore_state(mix_id_full_pledge, active_params);

                assert_eq!(dist1, dist2)
            }
        }

        #[test]
        fn updates_the_pending_pledge_changes_field() {
            let mut test = TestSetup::new();
            let mix_id = test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner"), None);
            test.set_pending_pledge_change(mix_id, None);

            let amount = test.coin(12345);
            decrease_mixnode_pledge(test.deps_mut(), 123, mix_id, amount).unwrap();
            let pending = mixnodes_storage::PENDING_MIXNODE_CHANGES
                .load(test.deps().storage, mix_id)
                .unwrap();
            assert!(pending.pledge_change.is_none())
        }
    }

    #[test]
    fn updating_active_set_updates_rewarding_params() {
        let mut test = TestSetup::new();
        let current = rewards_storage::REWARDING_PARAMS
            .load(test.deps().storage)
            .unwrap();

        update_active_set(
            test.deps_mut(),
            123,
            ActiveSetUpdate {
                entry_gateways: 50,
                exit_gateways: 50,
                mixnodes: 100,
            },
        )
        .unwrap();
        let updated = rewards_storage::REWARDING_PARAMS
            .load(test.deps().storage)
            .unwrap();
        assert_ne!(updated.rewarded_set, current.rewarded_set);
        assert_eq!(updated.rewarded_set.mixnodes, 100);
        assert_eq!(updated.rewarded_set.entry_gateways, 50);
        assert_eq!(updated.rewarded_set.exit_gateways, 50);
    }

    #[cfg(test)]
    mod changing_mix_cost_params {
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use cosmwasm_std::coin;

        use super::*;

        #[test]
        fn doesnt_do_anything_if_mixnode_has_unbonded() {
            let mut test = TestSetup::new();
            let mix_id = test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner"), None);

            let env = test.env();
            unbond_mixnode(test.deps_mut(), &env, 123, mix_id).unwrap();

            let new_params = NodeCostParams {
                profit_margin_percent: Percent::from_percentage_value(42).unwrap(),
                interval_operating_cost: coin(123_456_789, TEST_COIN_DENOM),
            };

            let res = change_mix_cost_params(test.deps_mut(), 123, mix_id, new_params);
            assert_eq!(res, Ok(Response::default()));
        }

        #[test]
        fn for_bonded_mixnode_updates_saved_value() {
            let mut test = TestSetup::new();
            let mix_id = test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner"), None);

            let before = test.mix_rewarding(mix_id).cost_params;

            // this would have been normally populated when creating the event itself
            mixnodes_storage::PENDING_MIXNODE_CHANGES
                .save(test.deps_mut().storage, mix_id, &Default::default())
                .unwrap();

            let new_params = NodeCostParams {
                profit_margin_percent: Percent::from_percentage_value(42).unwrap(),
                interval_operating_cost: coin(123_456_789, TEST_COIN_DENOM),
            };

            let res = change_mix_cost_params(test.deps_mut(), 123, mix_id, new_params.clone());
            assert_eq!(
                res,
                Ok(Response::new().add_event(new_cost_params_update_event(
                    123,
                    mix_id,
                    &new_params,
                )))
            );

            let after = test.mix_rewarding(mix_id).cost_params;
            assert_ne!(before, new_params);
            assert_eq!(after, new_params);
        }
    }

    #[test]
    fn updating_interval_rewarding_params_appropriately_recomputes_state() {
        let mut test = TestSetup::new();

        let before = rewards_storage::REWARDING_PARAMS
            .load(test.deps().storage)
            .unwrap();

        let two = Decimal::from_atomics(2u32, 0).unwrap();
        let four = Decimal::from_atomics(4u32, 0).unwrap();

        // TODO: be more fuzzy about it and try to vary other fields that can cause
        // re-computation like pool emission or rewarded set size update
        let update = IntervalRewardingParamsUpdate {
            reward_pool: Some(before.interval.reward_pool / two),
            staking_supply: Some(before.interval.staking_supply * four),
            staking_supply_scale_factor: None,
            sybil_resistance_percent: Some(Percent::from_percentage_value(42).unwrap()),
            active_set_work_factor: None,
            interval_pool_emission: None,
            rewarded_set_params: None,
        };

        let res = update_rewarding_params(test.deps_mut(), 123, update);
        assert!(res.is_ok());
        let after = rewards_storage::REWARDING_PARAMS
            .load(test.deps().storage)
            .unwrap();

        // with half the reward pool, our reward budget is also halved
        assert_decimals(
            before.interval.epoch_reward_budget,
            two * after.interval.epoch_reward_budget,
        );

        // and with 4x the staking supply, the saturation point is also increased 4-folds
        assert_decimals(
            four * before.interval.stake_saturation_point,
            after.interval.stake_saturation_point,
        );

        assert_eq!(
            after.interval.sybil_resistance,
            Percent::from_percentage_value(42).unwrap()
        )
    }

    #[test]
    fn updating_interval_config_recomputes_rewarding_params() {
        let mut test = TestSetup::new();

        let two = Decimal::from_atomics(2u32, 0).unwrap();

        let params_before = rewards_storage::REWARDING_PARAMS
            .load(test.deps().storage)
            .unwrap();

        // skip few epochs just for the sake of it
        test.skip_to_next_epoch();
        test.skip_to_next_epoch();
        test.skip_to_next_epoch();
        test.skip_to_next_epoch();
        test.skip_to_next_epoch();

        let interval_before =
            crate::interval::storage::current_interval(test.deps().storage).unwrap();

        // half the number of epochs (thus double reward budget)
        // and change epoch length
        update_interval_config(
            test.deps_mut(),
            123,
            interval_before.epochs_in_interval() / 2,
            1234,
        )
        .unwrap();

        let interval_after =
            crate::interval::storage::current_interval(test.deps().storage).unwrap();
        let params_after = rewards_storage::REWARDING_PARAMS
            .load(test.deps().storage)
            .unwrap();
        assert_eq!(
            interval_after.epochs_in_interval(),
            interval_before.epochs_in_interval() / 2
        );
        assert_eq!(
            params_after.interval.epoch_reward_budget,
            params_before.interval.epoch_reward_budget * two
        );
        assert_eq!(interval_after.epoch_length(), Duration::from_secs(1234))
    }
}

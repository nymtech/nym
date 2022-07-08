// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::delegations;
use crate::delegations::storage as delegations_storage;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::helpers::{cleanup_post_unbond_mixnode_storage, get_mixnode_details_by_id};
use crate::rewards::storage as rewards_storage;
use crate::support::helpers::send_to_proxy_or_owner;
use cosmwasm_std::{coin, coins, wasm_execute, Addr, Coin, Decimal, DepsMut, Env, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::mixnode::MixNodeCostParams;
use mixnet_contract_common::pending_events::{PendingEpochEvent, PendingIntervalEvent};
use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;
use mixnet_contract_common::{Delegation, NodeId};
use vesting_contract_common::messages::ExecuteMsg as VestingContractExecuteMsg;

pub(crate) trait ContractExecutableEvent {
    // note: the error only means a HARD error like we failed to read from storage.
    // if, for example, delegating fails because mixnode no longer exists, we return an Ok(()),
    // because it's not a hard error and we don't want to fail the entire transaction
    fn execute(self, deps: DepsMut<'_>, env: &Env) -> Result<Response, MixnetContractError>;
}

fn delegate(
    deps: DepsMut<'_>,
    env: &Env,
    owner: Addr,
    mix_id: NodeId,
    mut amount: Coin,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    // check if the target node still exists (it might have unbonded between this event getting created
    // and being executed). Do note that it's absolutely possible for a mixnode to get immediately
    // unbonded at this very block (if the event was pending), but that's tough luck, then it's up
    // to the delegator to click the undelegate button
    let mixnode_details = match get_mixnode_details_by_id(deps.storage, mix_id)? {
        Some(details)
            if details.rewarding_details.still_bonded()
                && !details.bond_information.is_unbonding =>
        {
            details
        }
        _ => {
            // if mixnode is no longer bonded or in the process of unbonding, return the tokens back to the
            // delegator;
            // TODO: do we need to do any vesting-specific tracking here?
            // to be figured out after undelegate is re-implemented
            let return_tokens = send_to_proxy_or_owner(&proxy, &owner, vec![amount]);
            return Ok(Response::new().add_message(return_tokens));
        }
    };

    let mut mix_rewarding = mixnode_details.rewarding_details;

    // if there's an existing delegation, then withdraw the full reward and create a new delegation
    // with the sum of both
    let storage_key = Delegation::generate_storage_key(mix_id, &owner, proxy.as_ref());
    let (amount, old_delegation) = if let Some(existing_delegation) =
        delegations_storage::delegations().may_load(deps.storage, storage_key.clone())?
    {
        // remove the reward from the node
        let reward = mix_rewarding.determine_delegation_reward(&existing_delegation);
        mix_rewarding.decrease_delegates(existing_delegation.dec_amount() + reward)?;

        // TODO: code duplication with 'undelegate'
        // if this is the only delegation, move all leftover decimal tokens to the operator
        // (this is literally in the order of a millionth of a micronym)
        if mix_rewarding.unique_delegations == 1 {
            mix_rewarding.operator += mix_rewarding.delegates;
            mix_rewarding.delegates = Decimal::zero();
        }

        let truncated_reward = truncate_reward_amount(reward);
        amount.amount += truncated_reward;

        (amount, Some(existing_delegation))
    } else {
        (amount, None)
    };

    // add the amount we're intending to delegate
    mix_rewarding.add_base_delegation(amount.amount);

    // create delegation and store it
    let delegation = Delegation::new(
        owner,
        mix_id,
        mix_rewarding.total_unit_reward,
        amount,
        env.block.height,
        proxy,
    );

    // save on reading since `.save()` would have attempted to read old data that we already have on hand
    delegations_storage::delegations().replace(
        deps.storage,
        storage_key,
        Some(&delegation),
        old_delegation.as_ref(),
    )?;
    rewards_storage::MIXNODE_REWARDING.save(deps.storage, mix_id, &mix_rewarding)?;

    Ok(Response::new())
}

fn undelegate(
    deps: DepsMut<'_>,
    _env: &Env,
    owner: Addr,
    mix_id: NodeId,
    proxy: Option<Addr>,
) -> Result<Response, MixnetContractError> {
    // see if the delegation still exists (in case of impatient user who decided to send multiple
    // undelegation requests in an epoch)
    let storage_key = Delegation::generate_storage_key(mix_id, &owner, proxy.as_ref());
    let delegation = match delegations_storage::delegations().may_load(deps.storage, storage_key)? {
        None => return Ok(Response::default()),
        Some(delegation) => delegation,
    };
    let mix_rewarding =
        rewards_storage::MIXNODE_REWARDING.may_load(deps.storage, mix_id)?.ok_or(MixnetContractError::InconsistentState {
            comment: "mixnode rewarding got removed from the storage whilst there's still an existing delegation"
                .into(),
        })?;

    // this also appropriately adjusts the storage
    let tokens_to_return =
        delegations::helpers::undelegate(deps.storage, delegation, mix_rewarding)?;

    let return_tokens = send_to_proxy_or_owner(&proxy, &owner, vec![tokens_to_return.clone()]);
    let mut response = Response::new().add_message(return_tokens);

    if let Some(proxy) = &proxy {
        // we can only attempt to send the message to the vesting contract if the proxy IS the vesting contract
        // otherwise, we don't care
        let vesting_contract = mixnet_params_storage::vesting_contract_address(deps.storage)?;
        if proxy == &vesting_contract {
            // TODO: do we need to send the 1ucoin here?
            // TODO2: why not just send the tokens alongside the execute?
            let min_coin = coins(1, &tokens_to_return.denom);

            let msg = VestingContractExecuteMsg::TrackUndelegation {
                owner: owner.clone().into_string(),
                mix_identity: "".to_string(),
                amount: tokens_to_return,
            };
            let msg = todo!("we no longer have mix_identity on hand -> this needs adjustments");
            let track_unbond_message = wasm_execute(proxy, &msg, min_coin)?;
            response = response.add_message(track_unbond_message);
        }
    }

    // TODO: slap events on it
    Ok(response)
}

fn unbond_mixnode(
    deps: DepsMut<'_>,
    _env: &Env,
    mix_id: NodeId,
) -> Result<Response, MixnetContractError> {
    // if we're here it means user executed `_try_remove_mixnode` and as a result node was set to be
    // in unbonding state and thus nothing could have been done to it (such as attempting to double unbond it)
    // thus the node with all its associated information MUST exist in the storage.
    let node_details = get_mixnode_details_by_id(deps.storage, mix_id)?.ok_or(
        MixnetContractError::InconsistentState {
            comment: "mixnode getting processed to get unbonded doesn't exist in the storage"
                .into(),
        },
    )?;

    // the denom on the original pledge was validated at the time of bonding so we can safely reuse it here
    let rewarding_denom = &node_details.bond_information.original_pledge.denom;
    let tokens = node_details
        .rewarding_details
        .operator_pledge_with_reward(rewarding_denom);

    let proxy = &node_details.bond_information.proxy;
    let owner = &node_details.bond_information.owner;

    // send bonded funds (alongside all earned rewards) to the bond owner
    let return_tokens = send_to_proxy_or_owner(proxy, owner, vec![tokens.clone()]);

    // remove the bond and if there are no delegations left, also the rewarding information
    // decrement the associated layer count
    cleanup_post_unbond_mixnode_storage(deps.storage, &node_details)?;

    let mut response = Response::new().add_message(return_tokens);

    if let Some(proxy) = &proxy {
        // we can only attempt to send the message to the vesting contract if the proxy IS the vesting contract
        // otherwise, we don't care
        let vesting_contract = mixnet_params_storage::vesting_contract_address(deps.storage)?;
        if proxy == &vesting_contract {
            let msg = VestingContractExecuteMsg::TrackUnbondMixnode {
                owner: owner.clone().into_string(),
                amount: tokens.clone(),
            };

            // TODO: do we need to send the 1ucoin here?
            let min_coin = coins(1, &tokens.denom);
            let track_unbond_message = wasm_execute(proxy, &msg, min_coin)?;
            response = response.add_message(track_unbond_message);
        }
    }

    // TODO: slap events on it
    Ok(response)
}

impl ContractExecutableEvent for PendingEpochEvent {
    fn execute(self, deps: DepsMut<'_>, env: &Env) -> Result<Response, MixnetContractError> {
        // note that the basic validation on all those events was already performed before
        // they were pushed onto the queue
        match self {
            PendingEpochEvent::Delegate {
                owner,
                mix_id,
                amount,
                proxy,
            } => delegate(deps, env, owner, mix_id, amount, proxy),
            PendingEpochEvent::Undelegate {
                owner,
                mix_id,
                proxy,
            } => undelegate(deps, env, owner, mix_id, proxy),
            PendingEpochEvent::UnbondMixnode { mix_id } => unbond_mixnode(deps, env, mix_id),
        }
    }
}

fn change_mix_cost_params(
    deps: DepsMut<'_>,
    _env: &Env,
    mix_id: NodeId,
    new_costs: MixNodeCostParams,
) -> Result<Response, MixnetContractError> {
    // almost an entire interval might have passed since the request was issued -> check if the
    // node still exists

    // note: there's no check if the bond is in "unbonding" state, as epoch actions would get
    // cleared before touching interval actions
    let mut mix_rewarding =
        match rewards_storage::MIXNODE_REWARDING.may_load(deps.storage, mix_id)? {
            Some(mix_rewarding) if mix_rewarding.still_bonded() => mix_rewarding,
            // if node doesn't exist anymore, don't do anything, simple as that.
            _ => return Ok(Response::default()),
        };
    // TODO: can we just change cost_params without breaking rewarding calculation?
    // (I'm almost certain we can, but well, it has to be tested)
    mix_rewarding.cost_params = new_costs;
    rewards_storage::MIXNODE_REWARDING.save(deps.storage, mix_id, &mix_rewarding)?;

    // TODO: slap events on it
    Ok(Response::new())
}

impl ContractExecutableEvent for PendingIntervalEvent {
    fn execute(self, deps: DepsMut<'_>, env: &Env) -> Result<Response, MixnetContractError> {
        // note that the basic validation on all those events was already performed before
        // they were pushed onto the queue
        match self {
            PendingIntervalEvent::ChangeMixCostParams { mix, new_costs } => {
                change_mix_cost_params(deps, env, mix, new_costs)
            }
        }
    }
}

// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::delegations;
use crate::delegations::storage as delegations_storage;
use crate::interval::helpers::change_epochs_in_interval;
use crate::interval::storage;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::helpers::{cleanup_post_unbond_mixnode_storage, get_mixnode_details_by_id};
use crate::rewards::storage as rewards_storage;
use crate::support::helpers::send_to_proxy_or_owner;
use cosmwasm_std::{wasm_execute, Addr, Coin, DepsMut, Env, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_active_set_update_event, new_delegation_event, new_delegation_on_unbonded_node_event,
    new_interval_config_update_event, new_mixnode_cost_params_update_event,
    new_mixnode_unbonding_event, new_rewarding_params_update_event, new_undelegation_event,
};
use mixnet_contract_common::mixnode::MixNodeCostParams;
use mixnet_contract_common::pending_events::{PendingEpochEvent, PendingIntervalEvent};
use mixnet_contract_common::reward_params::IntervalRewardingParamsUpdate;
use mixnet_contract_common::{Delegation, NodeId};
use std::time::Duration;
use vesting_contract_common::messages::ExecuteMsg as VestingContractExecuteMsg;

pub(crate) trait ContractExecutableEvent {
    // note: the error only means a HARD error like we failed to read from storage.
    // if, for example, delegating fails because mixnode no longer exists, we return an Ok(()),
    // because it's not a hard error and we don't want to fail the entire transaction
    fn execute(self, deps: DepsMut<'_>, env: &Env) -> Result<Response, MixnetContractError>;
}

pub(crate) fn delegate(
    deps: DepsMut<'_>,
    env: &Env,
    owner: Addr,
    mix_id: NodeId,
    amount: Coin,
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
            // TODO: @DU: do we need to do any vesting-specific tracking here?
            // to be figured out after undelegate is re-implemented
            let return_tokens = send_to_proxy_or_owner(&proxy, &owner, vec![amount.clone()]);
            let mut response = Response::new().add_message(return_tokens).add_event(
                new_delegation_on_unbonded_node_event(&owner, &proxy, mix_id),
            );

            if let Some(proxy) = &proxy {
                // we can only attempt to send the message to the vesting contract if the proxy IS the vesting contract
                // otherwise, we don't care
                let vesting_contract =
                    mixnet_params_storage::vesting_contract_address(deps.storage)?;
                if proxy == &vesting_contract {
                    let msg = VestingContractExecuteMsg::TrackUndelegation {
                        owner: owner.into_string(),
                        mix_id,
                        amount,
                    };

                    let track_undelegate_message = wasm_execute(proxy, &msg, vec![])?;
                    response = response.add_message(track_undelegate_message);
                }
            }

            return Ok(response);
        }
    };

    let new_delegation_amount = amount.clone();
    let mut mix_rewarding = mixnode_details.rewarding_details;

    // the delegation_amount might get increased if there's already a pre-existing delegation on this mixnode
    // (in that case we just create a fresh delegation with the sum of both)
    let mut stored_delegation_amount = amount;

    // if there's an existing delegation, then withdraw the full reward and create a new delegation
    // with the sum of both
    let storage_key = Delegation::generate_storage_key(mix_id, &owner, proxy.as_ref());
    let old_delegation = if let Some(existing_delegation) =
        delegations_storage::delegations().may_load(deps.storage, storage_key.clone())?
    {
        // completely remove the delegation from the node
        let og_with_reward = mix_rewarding.undelegate(&existing_delegation)?;

        // and adjust the new value by the amount removed (which contains the original delegation
        // alongside any earned rewards)
        stored_delegation_amount.amount += og_with_reward.amount;

        Some(existing_delegation)
    } else {
        None
    };

    // add the amount we're intending to delegate (whether it's fresh or we're adding to the existing one)
    mix_rewarding.add_base_delegation(stored_delegation_amount.amount);

    let cosmos_event = new_delegation_event(&owner, &proxy, &new_delegation_amount, mix_id);

    let delegation = Delegation::new(
        owner,
        mix_id,
        mix_rewarding.total_unit_reward,
        stored_delegation_amount,
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

    Ok(Response::new().add_event(cosmos_event))
}

pub(crate) fn undelegate(
    deps: DepsMut<'_>,
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
    let mut response = Response::new()
        .add_message(return_tokens)
        .add_event(new_undelegation_event(&owner, &proxy, mix_id));

    if let Some(proxy) = &proxy {
        // we can only attempt to send the message to the vesting contract if the proxy IS the vesting contract
        // otherwise, we don't care
        let vesting_contract = mixnet_params_storage::vesting_contract_address(deps.storage)?;
        if proxy == &vesting_contract {
            let msg = VestingContractExecuteMsg::TrackUndelegation {
                owner: owner.into_string(),
                mix_id,
                amount: tokens_to_return,
            };

            let track_undelegate_message = wasm_execute(proxy, &msg, vec![])?;
            response = response.add_message(track_undelegate_message);
        }
    }

    Ok(response)
}

pub(crate) fn unbond_mixnode(
    deps: DepsMut<'_>,
    env: &Env,
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
    cleanup_post_unbond_mixnode_storage(deps.storage, env, &node_details)?;

    let mut response = Response::new()
        .add_message(return_tokens)
        .add_event(new_mixnode_unbonding_event(mix_id));

    if let Some(proxy) = &proxy {
        // we can only attempt to send the message to the vesting contract if the proxy IS the vesting contract
        // otherwise, we don't care
        let vesting_contract = mixnet_params_storage::vesting_contract_address(deps.storage)?;
        if proxy == &vesting_contract {
            let msg = VestingContractExecuteMsg::TrackUnbondMixnode {
                owner: owner.clone().into_string(),
                amount: tokens,
            };

            let track_unbond_message = wasm_execute(proxy, &msg, vec![])?;
            response = response.add_message(track_unbond_message);
        }
    }

    Ok(response)
}

pub(crate) fn update_active_set_size(
    deps: DepsMut<'_>,
    active_set_size: u32,
) -> Result<Response, MixnetContractError> {
    // We don't have to check for authorization as this event can only be pushed
    // by the authorized entity.
    // Furthermore, we don't need to check whether the epoch is finished as the
    // queue is only emptied upon the epoch finishing.
    // Also, we know the update is valid as we checked for that before pushing the event onto the queue.

    let mut rewarding_params = rewards_storage::REWARDING_PARAMS.load(deps.storage)?;
    rewarding_params.try_change_active_set_size(active_set_size)?;
    rewards_storage::REWARDING_PARAMS.save(deps.storage, &rewarding_params)?;

    Ok(Response::new().add_event(new_active_set_update_event(active_set_size)))
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
            } => undelegate(deps, owner, mix_id, proxy),
            PendingEpochEvent::UnbondMixnode { mix_id } => unbond_mixnode(deps, env, mix_id),
            PendingEpochEvent::UpdateActiveSetSize { new_size } => {
                update_active_set_size(deps, new_size)
            }
        }
    }
}

pub(crate) fn change_mix_cost_params(
    deps: DepsMut<'_>,
    mix_id: NodeId,
    new_costs: MixNodeCostParams,
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

    let cosmos_event = new_mixnode_cost_params_update_event(mix_id, &new_costs);

    // TODO: can we just change cost_params without breaking rewarding calculation?
    // (I'm almost certain we can, but well, it has to be tested)
    mix_rewarding.cost_params = new_costs;
    rewards_storage::MIXNODE_REWARDING.save(deps.storage, mix_id, &mix_rewarding)?;

    Ok(Response::new().add_event(cosmos_event))
}

pub(crate) fn update_rewarding_params(
    deps: DepsMut<'_>,
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
        updated_params,
        rewarding_params.interval,
    )))
}

pub(crate) fn update_interval_config(
    deps: DepsMut,
    epochs_in_interval: u32,
    epoch_duration_secs: u64,
) -> Result<Response, MixnetContractError> {
    // We don't have to check for authorization as this event can only be pushed
    // by the authorized entity.
    // Furthermore, we don't need to check whether the interval is finished as the
    // queue is only emptied upon the interval finishing.
    let mut interval = storage::current_interval(deps.storage)?;
    interval.change_epoch_length(Duration::from_secs(epoch_duration_secs));
    change_epochs_in_interval(deps.storage, interval, epochs_in_interval)?;

    Ok(Response::new().add_event(new_interval_config_update_event(
        epochs_in_interval,
        epoch_duration_secs,
    )))
}

impl ContractExecutableEvent for PendingIntervalEvent {
    fn execute(self, deps: DepsMut<'_>, _env: &Env) -> Result<Response, MixnetContractError> {
        // note that the basic validation on all those events was already performed before
        // they were pushed onto the queue
        match self {
            PendingIntervalEvent::ChangeMixCostParams { mix, new_costs } => {
                change_mix_cost_params(deps, mix, new_costs)
            }
            PendingIntervalEvent::UpdateRewardingParams { update } => {
                update_rewarding_params(deps, update)
            }
            PendingIntervalEvent::UpdateIntervalConfig {
                epochs_in_interval,
                epoch_duration_secs,
            } => update_interval_config(deps, epochs_in_interval, epoch_duration_secs),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::test_helpers;
    use crate::support::tests::test_helpers::{assert_decimals, TestSetup};
    use cosmwasm_std::Decimal;
    use mixnet_contract_common::Percent;

    // note that authorization and basic validation has already been performed for all of those
    // before being pushed onto the event queues

    #[cfg(test)]
    mod delegating {
        use super::*;
        use crate::mixnodes::transactions::try_remove_mixnode;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use crate::support::tests::test_helpers::get_bank_send_msg;
        use cosmwasm_std::testing::mock_info;
        use cosmwasm_std::{coin, to_binary, CosmosMsg, Decimal, WasmMsg};
        use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;

        #[test]
        fn returns_the_tokens_if_mixnode_has_unbonded() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let delegation = 120_000_000u128;
            let delegation_coin = coin(delegation, TEST_COIN_DENOM);
            let owner1 = "delegator1";
            let owner2 = "delegator2";

            // add pre-existing delegation
            test.add_immediate_delegation(owner1, delegation, mix_id);

            let env = test.env();
            unbond_mixnode(test.deps_mut(), &env, mix_id).unwrap();

            let res_increase = delegate(
                test.deps_mut(),
                &env,
                Addr::unchecked(owner1),
                mix_id,
                delegation_coin.clone(),
                None,
            )
            .unwrap();

            // delegation wasn't increased
            let storage_key =
                Delegation::generate_storage_key(mix_id, &Addr::unchecked(owner1), None);
            let amount = delegations_storage::delegations()
                .load(test.deps().storage, storage_key)
                .unwrap()
                .amount;
            assert_eq!(amount, delegation_coin);

            // and all tokens are returned back to the delegator
            let (receiver, sent_amount) = get_bank_send_msg(&res_increase).unwrap();
            assert_eq!(receiver, owner1);
            assert_eq!(sent_amount[0], delegation_coin);

            // for a fresh delegation, nothing was added to the storage either
            let res_fresh = delegate(
                test.deps_mut(),
                &env,
                Addr::unchecked(owner2),
                mix_id,
                delegation_coin.clone(),
                None,
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
            assert_eq!(receiver, owner2);
            assert_eq!(sent_amount[0], delegation_coin);
        }

        #[test]
        fn returns_the_tokens_is_mixnode_is_unbonding() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let delegation = 120_000_000u128;
            let delegation_coin = coin(delegation, TEST_COIN_DENOM);
            let owner1 = "delegator1";
            let owner2 = "delegator2";

            // add pre-existing delegation
            test.add_immediate_delegation(owner1, delegation, mix_id);

            let env = test.env();
            try_remove_mixnode(test.deps_mut(), mock_info("mix-owner", &[])).unwrap();

            let res_increase = delegate(
                test.deps_mut(),
                &env,
                Addr::unchecked(owner1),
                mix_id,
                delegation_coin.clone(),
                None,
            )
            .unwrap();

            // delegation wasn't increased
            let storage_key =
                Delegation::generate_storage_key(mix_id, &Addr::unchecked(owner1), None);
            let amount = delegations_storage::delegations()
                .load(test.deps().storage, storage_key)
                .unwrap()
                .amount;
            assert_eq!(amount, delegation_coin);

            // and all tokens are returned back to the delegator
            let (receiver, sent_amount) = get_bank_send_msg(&res_increase).unwrap();
            assert_eq!(receiver, owner1);
            assert_eq!(sent_amount[0], delegation_coin);

            // for a fresh delegation, nothing was added to the storage either
            let res_fresh = delegate(
                test.deps_mut(),
                &env,
                Addr::unchecked(owner2),
                mix_id,
                delegation_coin.clone(),
                None,
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
            assert_eq!(receiver, owner2);
            assert_eq!(sent_amount[0], delegation_coin);
        }

        #[test]
        fn if_delegation_already_exists_a_fresh_one_with_sum_of_both_is_created() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", Some(100_000_000_000u128.into()));

            let delegation_og = 120_000_000u128;
            let delegation_new = 543_000_000u128;
            let delegation_coin_new = coin(delegation_new, TEST_COIN_DENOM);

            let owner = "delegator";
            test.add_immediate_delegation(owner, delegation_og, mix_id);

            let env = test.env();
            let res = delegate(
                test.deps_mut(),
                &env,
                Addr::unchecked(owner),
                mix_id,
                delegation_coin_new,
                None,
            )
            .unwrap();

            let expected_amount = delegation_og + delegation_new;
            let expected_amount_dec = Decimal::from_atomics(expected_amount, 0).unwrap();

            // no refunds here!
            assert!(get_bank_send_msg(&res).is_none());

            let rewarding = rewards_storage::MIXNODE_REWARDING
                .load(test.deps().storage, mix_id)
                .unwrap();
            let storage_key =
                Delegation::generate_storage_key(mix_id, &Addr::unchecked(owner), None);
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
            let mix_id = test.add_dummy_mixnode("mix-owner", Some(100_000_000_000u128.into()));

            let delegation_og = 120_000_000u128;
            let delegation_new = 543_000_000u128;
            let delegation_coin_new = coin(delegation_new, TEST_COIN_DENOM);

            // perform some rewarding here to advance the unit delegation beyond the initial value
            test.update_rewarded_set(vec![mix_id]);
            test.skip_to_next_epoch_end();
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));
            test.skip_to_next_epoch_end();
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

            let owner = "delegator";
            test.add_immediate_delegation(owner, delegation_og, mix_id);

            test.skip_to_next_epoch_end();
            let dist1 = test.reward_with_distribution(mix_id, test_helpers::performance(100.0));
            test.skip_to_next_epoch_end();
            let dist2 = test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

            let storage_key =
                Delegation::generate_storage_key(mix_id, &Addr::unchecked(owner), None);
            let delegation_pre = delegations_storage::delegations()
                .load(test.deps().storage, storage_key.clone())
                .unwrap();

            let env = test.env();
            let res = delegate(
                test.deps_mut(),
                &env,
                Addr::unchecked(owner),
                mix_id,
                delegation_coin_new,
                None,
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
            let mix_id = test.add_dummy_mixnode("mix-owner", Some(100_000_000_000u128.into()));
            let owner = "delegator";

            let delegation = 120_000_000u128;
            let delegation_coin = coin(120_000_000u128, TEST_COIN_DENOM);

            // perform some rewarding here to advance the unit delegation beyond the initial value
            test.update_rewarded_set(vec![mix_id]);
            test.skip_to_next_epoch_end();
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));
            test.skip_to_next_epoch_end();
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

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
                Addr::unchecked(owner),
                mix_id,
                delegation_coin.clone(),
                None,
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

        #[test]
        fn attaches_vesting_contract_track_message_if_tokens_are_returned() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let delegation = 120_000_000u128;
            let delegation_coin = coin(delegation, TEST_COIN_DENOM);
            let owner1 = "delegator1";
            let owner2 = "delegator2";

            let env = test.env();
            unbond_mixnode(test.deps_mut(), &env, mix_id).unwrap();

            let vesting_contract = test.vesting_contract();
            let dummy_proxy = Addr::unchecked("not-vesting-contract");

            // for a fresh delegation, nothing was added to the storage either
            let res_vesting = delegate(
                test.deps_mut(),
                &env,
                Addr::unchecked(owner1),
                mix_id,
                delegation_coin.clone(),
                Some(vesting_contract.clone()),
            )
            .unwrap();
            let storage_key = Delegation::generate_storage_key(
                mix_id,
                &Addr::unchecked(owner1),
                Some(vesting_contract.clone()).as_ref(),
            );
            assert!(delegations_storage::delegations()
                .may_load(test.deps().storage, storage_key)
                .unwrap()
                .is_none());
            // and all tokens are returned back to the proxy
            let (receiver, sent_amount) = get_bank_send_msg(&res_vesting).unwrap();
            assert_eq!(receiver, vesting_contract.as_str());
            assert_eq!(sent_amount[0], delegation_coin);

            // and we get appropriate track message
            let mut found_track = true;
            for msg in &res_vesting.messages {
                if let CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr,
                    msg,
                    funds,
                }) = &msg.msg
                {
                    found_track = true;
                    assert_eq!(contract_addr, vesting_contract.as_str());
                    let expected_msg = to_binary(&VestingContractExecuteMsg::TrackUndelegation {
                        owner: owner1.to_string(),
                        mix_id,
                        amount: delegation_coin.clone(),
                    })
                    .unwrap();
                    assert_eq!(&expected_msg, msg);
                    assert!(funds.is_empty())
                }
            }
            assert!(found_track);

            let res_other_proxy = delegate(
                test.deps_mut(),
                &env,
                Addr::unchecked(owner1),
                mix_id,
                delegation_coin.clone(),
                Some(dummy_proxy.clone()),
            )
            .unwrap();
            let storage_key = Delegation::generate_storage_key(
                mix_id,
                &Addr::unchecked(owner2),
                Some(dummy_proxy.clone()).as_ref(),
            );
            assert!(delegations_storage::delegations()
                .may_load(test.deps().storage, storage_key)
                .unwrap()
                .is_none());
            // and all tokens are returned back to the proxy
            let (receiver, sent_amount) = get_bank_send_msg(&res_other_proxy).unwrap();
            assert_eq!(receiver, dummy_proxy.as_str());
            assert_eq!(sent_amount[0], delegation_coin);

            // no track messages here!
            for msg in &res_other_proxy.messages {
                if let CosmosMsg::Wasm(WasmMsg::Execute { .. }) = &msg.msg {
                    panic!("we shouldn't have attempted to call any other contract!")
                }
            }
        }
    }

    #[cfg(test)]
    mod undelegating {
        use super::*;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use crate::support::tests::test_helpers::get_bank_send_msg;
        use cosmwasm_std::{coin, to_binary, CosmosMsg, WasmMsg};
        use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;

        #[test]
        fn doesnt_return_any_tokens_if_it_doesnt_exist() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let owner = Addr::unchecked("delegator");

            let res = undelegate(test.deps_mut(), owner, mix_id, None).unwrap();
            assert!(get_bank_send_msg(&res).is_none());
        }

        #[test]
        fn errors_out_if_mix_rewarding_doesnt_exist() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let owner = Addr::unchecked("delegator");
            test.add_immediate_delegation(owner.as_str(), 100_000_000u32, mix_id);

            // this should never happen in actual code, but if we manually messed something up,
            // lets make sure this throws an error
            rewards_storage::MIXNODE_REWARDING.remove(test.deps_mut().storage, mix_id);
            let res = undelegate(test.deps_mut(), owner, mix_id, None);
            assert!(matches!(
                res,
                Err(MixnetContractError::InconsistentState { .. })
            ));
        }

        #[test]
        fn returns_all_delegated_tokens_with_earned_rewards() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", Some(100_000_000_000u128.into()));

            let owner = "delegator";
            let delegation = 120_000_000u128;

            // perform some rewarding here to advance the unit delegation beyond the initial value
            test.update_rewarded_set(vec![mix_id]);
            test.skip_to_next_epoch_end();
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));
            test.skip_to_next_epoch_end();
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

            test.add_immediate_delegation(owner, delegation, mix_id);

            test.skip_to_next_epoch_end();
            let dist1 = test.reward_with_distribution(mix_id, test_helpers::performance(100.0));
            test.skip_to_next_epoch_end();
            let dist2 = test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

            let expected_reward = dist1.delegates + dist2.delegates;
            let truncated_reward = truncate_reward_amount(expected_reward);

            let expected_return = delegation + truncated_reward.u128();

            let res = undelegate(test.deps_mut(), Addr::unchecked(owner), mix_id, None).unwrap();
            let (receiver, sent_amount) = get_bank_send_msg(&res).unwrap();
            assert_eq!(receiver, owner);
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

        #[test]
        fn attaches_vesting_contract_track_message_if_tokens_are_returned() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let delegation = 120_000_000u128;
            let delegation_coin = coin(delegation, TEST_COIN_DENOM);
            let owner1 = "delegator1";
            let owner2 = "delegator2";

            let vesting_contract = test.vesting_contract();
            let dummy_proxy = Addr::unchecked("not-vesting-contract");

            test.add_immediate_delegation_with_proxy(
                owner1,
                delegation,
                mix_id,
                vesting_contract.clone(),
            );
            test.add_immediate_delegation_with_proxy(
                owner2,
                delegation,
                mix_id,
                dummy_proxy.clone(),
            );

            // for a fresh delegation, nothing was added to the storage either
            let res_vesting = undelegate(
                test.deps_mut(),
                Addr::unchecked(owner1),
                mix_id,
                Some(vesting_contract.clone()),
            )
            .unwrap();
            let storage_key = Delegation::generate_storage_key(
                mix_id,
                &Addr::unchecked(owner1),
                Some(vesting_contract.clone()).as_ref(),
            );
            assert!(delegations_storage::delegations()
                .may_load(test.deps().storage, storage_key)
                .unwrap()
                .is_none());

            // and all tokens are returned back to the proxy
            let (receiver, sent_amount) = get_bank_send_msg(&res_vesting).unwrap();
            assert_eq!(receiver, vesting_contract.as_str());
            assert_eq!(sent_amount[0], delegation_coin);

            // and we get appropriate track message
            let mut found_track = true;
            for msg in &res_vesting.messages {
                if let CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr,
                    msg,
                    funds,
                }) = &msg.msg
                {
                    found_track = true;
                    assert_eq!(contract_addr, vesting_contract.as_str());
                    let expected_msg = to_binary(&VestingContractExecuteMsg::TrackUndelegation {
                        owner: owner1.to_string(),
                        mix_id,
                        amount: delegation_coin.clone(),
                    })
                    .unwrap();
                    assert_eq!(&expected_msg, msg);
                    assert!(funds.is_empty())
                }
            }
            assert!(found_track);

            let res_other_proxy = undelegate(
                test.deps_mut(),
                Addr::unchecked(owner2),
                mix_id,
                Some(dummy_proxy.clone()),
            )
            .unwrap();
            let storage_key = Delegation::generate_storage_key(
                mix_id,
                &Addr::unchecked(owner2),
                Some(dummy_proxy.clone()).as_ref(),
            );
            assert!(delegations_storage::delegations()
                .may_load(test.deps().storage, storage_key)
                .unwrap()
                .is_none());
            // and all tokens are returned back to the proxy
            let (receiver, sent_amount) = get_bank_send_msg(&res_other_proxy).unwrap();
            assert_eq!(receiver, dummy_proxy.as_str());
            assert_eq!(sent_amount[0], delegation_coin);

            // no track messages here!
            for msg in &res_other_proxy.messages {
                if let CosmosMsg::Wasm(WasmMsg::Execute { .. }) = &msg.msg {
                    panic!("we shouldn't have attempted to call any other contract!")
                }
            }
        }
    }

    #[cfg(test)]
    mod mixnode_unbonding {
        use super::*;
        use crate::mixnodes::storage as mixnodes_storage;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use crate::support::tests::test_helpers::get_bank_send_msg;
        use cosmwasm_std::{coin, to_binary, CosmosMsg, Uint128, WasmMsg};
        use mixnet_contract_common::mixnode::UnbondedMixnode;
        use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;

        #[test]
        fn returns_hard_error_if_mixnode_doesnt_exist() {
            // this should have never happened so hard error MUST be thrown here
            let mut test = TestSetup::new();
            let env = test.env();

            let res = unbond_mixnode(test.deps_mut(), &env, 1);
            assert!(matches!(
                res,
                Err(MixnetContractError::InconsistentState { .. })
            ));
        }

        #[test]
        fn returns_original_pledge_alongside_any_earned_rewards() {
            let mut test = TestSetup::new();

            let owner = "mix-owner";
            let pledge = Uint128::new(250_000_000);
            let mix_id = test.add_dummy_mixnode(owner, Some(pledge));
            let mix_details = mixnodes_storage::mixnode_bonds()
                .load(test.deps().storage, mix_id)
                .unwrap();
            let layer = mix_details.layer;

            test.update_rewarded_set(vec![mix_id]);
            test.skip_to_next_epoch_end();
            let dist1 = test.reward_with_distribution(mix_id, test_helpers::performance(100.0));
            test.skip_to_next_epoch_end();
            let dist2 = test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

            let expected_reward = dist1.operator + dist2.operator;
            let truncated_reward = truncate_reward_amount(expected_reward);
            let expected_return = pledge + truncated_reward;

            let env = test.env();
            let res = unbond_mixnode(test.deps_mut(), &env, mix_id).unwrap();
            let (receiver, sent_amount) = get_bank_send_msg(&res).unwrap();
            assert_eq!(receiver, owner);
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
                identity: mix_details.identity().to_string(),
                owner: Addr::unchecked(owner),
                unbonding_height: env.block.height,
            };
            assert_eq!(
                expected,
                mixnodes_storage::UNBONDED_MIXNODES
                    .load(test.deps().storage, mix_id)
                    .unwrap()
            );
            assert_eq!(
                mixnodes_storage::LAYERS.load(test.deps().storage).unwrap()[layer],
                0
            )
        }

        #[test]
        fn attaches_vesting_contract_track_message_if_tokens_are_returned() {
            let mut test = TestSetup::new();

            let vesting_contract = test.vesting_contract();
            let dummy_proxy = Addr::unchecked("not-vesting-contract");

            let pledge = Uint128::new(250_000_000);
            let pledge_coin = coin(250_000_000, TEST_COIN_DENOM);
            let owner1 = "mix-owner1";
            let owner2 = "mix-owner2";
            let mix_id_vesting =
                test.add_dummy_mixnode_with_proxy(owner1, Some(pledge), vesting_contract.clone());
            let mix_id_other_proxy =
                test.add_dummy_mixnode_with_proxy(owner2, Some(pledge), dummy_proxy.clone());

            let env = test.env();
            let res_vesting = unbond_mixnode(test.deps_mut(), &env, mix_id_vesting).unwrap();

            assert!(mixnodes_storage::mixnode_bonds()
                .may_load(test.deps().storage, mix_id_vesting)
                .unwrap()
                .is_none());

            // and all tokens are returned back to the proxy
            let (receiver, sent_amount) = get_bank_send_msg(&res_vesting).unwrap();
            assert_eq!(receiver, vesting_contract.as_str());
            assert_eq!(sent_amount[0], pledge_coin);

            // and we get appropriate track message
            let mut found_track = true;
            for msg in &res_vesting.messages {
                if let CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr,
                    msg,
                    funds,
                }) = &msg.msg
                {
                    found_track = true;
                    assert_eq!(contract_addr, vesting_contract.as_str());
                    let expected_msg = to_binary(&VestingContractExecuteMsg::TrackUnbondMixnode {
                        owner: owner1.to_string(),
                        amount: pledge_coin.clone(),
                    })
                    .unwrap();
                    assert_eq!(&expected_msg, msg);
                    assert!(funds.is_empty())
                }
            }
            assert!(found_track);

            let res_other_proxy =
                unbond_mixnode(test.deps_mut(), &env, mix_id_other_proxy).unwrap();
            assert!(mixnodes_storage::mixnode_bonds()
                .may_load(test.deps().storage, mix_id_other_proxy)
                .unwrap()
                .is_none());
            // and all tokens are returned back to the proxy
            let (receiver, sent_amount) = get_bank_send_msg(&res_other_proxy).unwrap();
            assert_eq!(receiver, dummy_proxy.as_str());
            assert_eq!(sent_amount[0], pledge_coin);

            // no track messages here!
            for msg in &res_other_proxy.messages {
                if let CosmosMsg::Wasm(WasmMsg::Execute { .. }) = &msg.msg {
                    panic!("we shouldn't have attempted to call any other contract!")
                }
            }
        }
    }

    #[test]
    fn updating_active_set_updates_rewarding_params() {
        let mut test = TestSetup::new();
        let current = rewards_storage::REWARDING_PARAMS
            .load(test.deps().storage)
            .unwrap();

        update_active_set_size(test.deps_mut(), 50).unwrap();
        let updated = rewards_storage::REWARDING_PARAMS
            .load(test.deps().storage)
            .unwrap();
        assert_ne!(current.active_set_size, updated.active_set_size);
        assert_eq!(updated.active_set_size, 50)
    }

    #[cfg(test)]
    mod changing_mix_cost_params {
        use super::*;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use cosmwasm_std::coin;
        use mixnet_contract_common::Percent;

        #[test]
        fn doesnt_do_anything_if_mixnode_has_unbonded() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let env = test.env();
            unbond_mixnode(test.deps_mut(), &env, mix_id).unwrap();

            let new_params = MixNodeCostParams {
                profit_margin_percent: Percent::from_percentage_value(42).unwrap(),
                interval_operating_cost: coin(123_456_789, TEST_COIN_DENOM),
            };

            let res = change_mix_cost_params(test.deps_mut(), mix_id, new_params);
            assert_eq!(res, Ok(Response::default()));
        }

        #[test]
        fn for_bonded_mixnode_updates_saved_value() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            let before = test.mix_rewarding(mix_id).cost_params;

            let env = test.env();
            let new_params = MixNodeCostParams {
                profit_margin_percent: Percent::from_percentage_value(42).unwrap(),
                interval_operating_cost: coin(123_456_789, TEST_COIN_DENOM),
            };

            let res = change_mix_cost_params(test.deps_mut(), mix_id, new_params.clone());
            assert_eq!(
                res,
                Ok(Response::new()
                    .add_event(new_mixnode_cost_params_update_event(mix_id, &new_params)))
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
            sybil_resistance_percent: Some(Percent::from_percentage_value(42).unwrap()),
            active_set_work_factor: None,
            interval_pool_emission: None,
            rewarded_set_size: None,
        };

        let res = update_rewarding_params(test.deps_mut(), update);
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

// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::interval::helpers::change_interval_config;
use crate::interval::pending_events::ContractExecutableEvent;
use crate::interval::storage::push_new_interval_event;
use crate::mixnodes::transactions::update_mixnode_layer;
use crate::rewards;
use crate::rewards::storage as rewards_storage;
use crate::support::helpers::{
    ensure_can_advance_epoch, ensure_epoch_in_progress_state, ensure_is_authorized, ensure_is_owner,
};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Order, Response, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_advance_epoch_event, new_epoch_transition_start_event,
    new_pending_epoch_events_execution_event, new_pending_interval_config_update_event,
    new_pending_interval_events_execution_event, new_reconcile_pending_events,
};
use mixnet_contract_common::pending_events::PendingIntervalEventKind;
use mixnet_contract_common::{EpochState, EpochStatus, LayerAssignment, MixId};
use std::collections::BTreeSet;

// those two should be called in separate tx (from advancing epoch),
// since there might be a lot of events to execute.
// however, it should also be called when advancing epoch itself in case somebody
// managed to sneak in a transaction between those two operations
// (but then the amount of work is going to be minimal)
pub(crate) fn perform_pending_epoch_actions(
    mut deps: DepsMut<'_>,
    env: &Env,
    limit: Option<u32>,
) -> Result<(Response, u32), MixnetContractError> {
    let last_executed = storage::LAST_PROCESSED_EPOCH_EVENT.load(deps.storage)?;
    let last_inserted = storage::EPOCH_EVENT_ID_COUNTER.load(deps.storage)?;

    // no pending events
    if last_executed == last_inserted {
        return Ok((Response::new(), 0));
    }

    let pending = last_inserted - last_executed;
    let last = limit
        .map(|limit| {
            if limit >= pending {
                last_inserted
            } else {
                last_executed + limit
            }
        })
        .unwrap_or(last_inserted);

    let mut response = Response::new();
    // no need to use the [cosmwasm] range iterator as we know the exact keys in order
    for event_id in last_executed + 1..=last {
        let event = storage::PENDING_EPOCH_EVENTS.load(deps.storage, event_id)?;
        let mut sub_response = event.execute(deps.branch(), env)?;
        response.messages.append(&mut sub_response.messages);
        response.attributes.append(&mut sub_response.attributes);
        response.events.append(&mut sub_response.events);
        // response.data.append(&mut sub_response.data);

        storage::PENDING_EPOCH_EVENTS.remove(deps.storage, event_id);
    }

    storage::LAST_PROCESSED_EPOCH_EVENT.save(deps.storage, &last)?;

    Ok((response, last - last_executed))
}

pub(crate) fn perform_pending_interval_actions(
    mut deps: DepsMut<'_>,
    env: &Env,
    limit: Option<u32>,
) -> Result<(Response, u32), MixnetContractError> {
    let last_executed = storage::LAST_PROCESSED_INTERVAL_EVENT.load(deps.storage)?;
    let last_inserted = storage::INTERVAL_EVENT_ID_COUNTER.load(deps.storage)?;

    // no pending events
    if last_executed == last_inserted {
        return Ok((Response::new(), 0));
    }

    let pending = last_inserted - last_executed;
    let last = limit
        .map(|limit| {
            if limit >= pending {
                last_inserted
            } else {
                last_executed + limit
            }
        })
        .unwrap_or(last_inserted);

    let mut response = Response::new();
    // no need to use the [cosmwasm] range iterator as we know the exact keys in order
    for event_id in last_executed + 1..=last {
        let event = storage::PENDING_INTERVAL_EVENTS.load(deps.storage, event_id)?;
        let mut sub_response = event.execute(deps.branch(), env)?;
        response.messages.append(&mut sub_response.messages);
        response.attributes.append(&mut sub_response.attributes);
        response.events.append(&mut sub_response.events);
        // response.data.append(&mut sub_response.data);

        storage::PENDING_INTERVAL_EVENTS.remove(deps.storage, event_id);
    }

    storage::LAST_PROCESSED_INTERVAL_EVENT.save(deps.storage, &last)?;

    Ok((response, last - last_executed))
}

pub fn try_reconcile_epoch_events(
    mut deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    mut limit: Option<u32>,
) -> Result<Response, MixnetContractError> {
    // we need to ensure the request actually comes from the rewarding validator. otherwise the following would be possible:
    // - epoch has just finished (i.e. it's possible to call the reconcile function)
    // - the validator API is ABOUT to start rewarding
    // - somebody sneaks in some extra delegations
    // - the same person decides to pay the transaction fees and reconcile epoch events themselves
    // - the validator API distributes the rewards -> this new sneaky delegation is now included in reward calculation!
    let mut current_epoch_status = ensure_can_advance_epoch(&info.sender, deps.storage)?;
    current_epoch_status.ensure_is_in_event_reconciliation_state()?;

    let mut response = Response::new().add_event(new_reconcile_pending_events());

    let interval = storage::current_interval(deps.storage)?;
    if !interval.is_current_epoch_over(&env) {
        // if the current epoch is in progress, so must be the interval so there's no need to check that
        return Err(MixnetContractError::EpochInProgress {
            current_block_time: env.block.time.seconds(),
            epoch_start: interval.current_epoch_start_unix_timestamp(),
            epoch_end: interval.current_epoch_end_unix_timestamp(),
        });
    } else {
        let (mut sub_response, executed) =
            perform_pending_epoch_actions(deps.branch(), &env, limit)?;
        response.messages.append(&mut sub_response.messages);
        response.attributes.append(&mut sub_response.attributes);
        response.events.append(&mut sub_response.events);
        response
            .events
            .push(new_pending_epoch_events_execution_event(executed));

        limit = limit.map(|l| l - executed)
    }

    if interval.is_current_interval_over(&env) {
        // first clear epoch events queue and then touch the interval actions
        let (mut sub_response, executed) =
            perform_pending_interval_actions(deps.branch(), &env, limit)?;
        response.messages.append(&mut sub_response.messages);
        response.attributes.append(&mut sub_response.attributes);
        response.events.append(&mut sub_response.events);
        response
            .events
            .push(new_pending_interval_events_execution_event(executed));
    }

    // if there are no more events to clear, go into the next state
    let pending_events = super::queries::query_number_of_pending_events(deps.as_ref())?;
    // we can only progress if there are no epoch events AND if the interval has finished, that there are no interval events
    let progress = if pending_events.epoch_events == 0 {
        if interval.is_current_interval_over(&env) {
            pending_events.interval_events == 0
        } else {
            true
        }
    } else {
        false
    };

    if progress {
        current_epoch_status.state = EpochState::AdvancingEpoch;
        storage::save_current_epoch_status(deps.storage, &current_epoch_status)?;
    }

    Ok(response)
}

fn update_rewarded_set(
    storage: &mut dyn Storage,
    new_rewarded_set: Vec<MixId>,
    expected_active_set_size: u32,
) -> Result<(), MixnetContractError> {
    let reward_params = rewards_storage::REWARDING_PARAMS.load(storage)?;

    // the rewarded set has been determined based off active set size taken from the contract,
    // thus the expected value HAS TO match
    if expected_active_set_size != reward_params.active_set_size {
        return Err(MixnetContractError::UnexpectedActiveSetSize {
            received: expected_active_set_size,
            expected: reward_params.active_set_size,
        });
    }

    if new_rewarded_set.len() as u32 > reward_params.rewarded_set_size {
        return Err(MixnetContractError::UnexpectedRewardedSetSize {
            received: new_rewarded_set.len() as u32,
            expected: reward_params.rewarded_set_size,
        });
    }

    // check for duplicates
    let mut tmp_set = BTreeSet::new();
    for node_id in &new_rewarded_set {
        if !tmp_set.insert(node_id) {
            return Err(MixnetContractError::DuplicateRewardedSetNode { mix_id: *node_id });
        }
    }

    Ok(storage::update_rewarded_set(
        storage,
        expected_active_set_size,
        new_rewarded_set,
    )?)
}

pub fn try_begin_epoch_transition(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, MixnetContractError> {
    // Only the rewarding validator(s) can attempt to advance epoch
    ensure_is_authorized(&info.sender, deps.storage)?;

    // can't do pre-mature epoch transition...
    let current_interval = storage::current_interval(deps.storage)?;
    if !current_interval.is_current_epoch_over(&env) {
        return Err(MixnetContractError::EpochInProgress {
            current_block_time: env.block.time.seconds(),
            epoch_start: current_interval.current_epoch_start_unix_timestamp(),
            epoch_end: current_interval.current_epoch_end_unix_timestamp(),
        });
    }

    // ensure some other validator (currently not a problem), hasn't already committed to epoch progression
    ensure_epoch_in_progress_state(deps.storage)?;

    // Note: if at any point we decide to change our rewarded set to be few thousand nodes
    // and the below call fails, we'll have to pass `last_node_in_rewarded_set` as an argument to this function
    // and then verify whether the provided value is valid (by using range iterator on `REWARDED_SET`
    // and checking if there are any other entries following the provided value)
    let rewarded_set = storage::REWARDED_SET
        .range(deps.storage, None, None, Order::Ascending)
        .map(|kv| kv.map(|kv| kv.0))
        .collect::<Result<Vec<_>, _>>()?;

    // if there are no nodes to reward (i.e. empty rewarded set), we go straight into event reconciliation
    let new_epoch_state = if let Some(last) = rewarded_set.last() {
        EpochState::Rewarding {
            last_rewarded: 0,
            final_node_id: *last,
        }
    } else {
        EpochState::ReconcilingEvents
    };

    // progress into the first stage of epoch progression
    let new_epoch_status = EpochStatus {
        being_advanced_by: info.sender,
        state: new_epoch_state,
    };

    storage::save_current_epoch_status(deps.storage, &new_epoch_status)?;
    Ok(Response::new().add_event(new_epoch_transition_start_event(current_interval)))
}

pub fn try_advance_epoch(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    layer_assignments: Vec<LayerAssignment>,
    expected_active_set_size: u32,
) -> Result<Response, MixnetContractError> {
    // Only rewarding validator can attempt to advance epoch
    let mut current_epoch_status = ensure_can_advance_epoch(&info.sender, deps.storage)?;
    current_epoch_status.ensure_is_in_advancement_state()?;

    // we must make sure that we roll into new epoch / interval with up to date state
    // with no pending actions (like somebody wanting to update their profit margin)
    let current_interval = storage::current_interval(deps.storage)?;
    if !current_interval.is_current_epoch_over(&env) {
        return Err(MixnetContractError::EpochInProgress {
            current_block_time: env.block.time.seconds(),
            epoch_start: current_interval.current_epoch_start_unix_timestamp(),
            epoch_end: current_interval.current_epoch_end_unix_timestamp(),
        });
    }

    // if the current interval is over, apply reward pool changes
    if current_interval.is_current_interval_over(&env) {
        // this one is a very important one!
        rewards::helpers::apply_reward_pool_changes(deps.storage)?;
    }

    let updated_interval = current_interval.advance_epoch();
    let num_nodes = layer_assignments.len();

    let new_rewarded_set = layer_assignments.iter().map(|l| l.mix_id()).collect();

    // finally save updated interval and the rewarded set
    storage::save_interval(deps.storage, &updated_interval)?;
    update_rewarded_set(deps.storage, new_rewarded_set, expected_active_set_size)?;

    for a in layer_assignments {
        update_mixnode_layer(a.mix_id(), a.layer(), deps.storage)?;
    }

    current_epoch_status.state = EpochState::InProgress;
    storage::save_current_epoch_status(deps.storage, &current_epoch_status)?;

    Ok(Response::new().add_event(new_advance_epoch_event(updated_interval, num_nodes as u32)))
}

pub(crate) fn try_update_interval_config(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    epochs_in_interval: u32,
    epoch_duration_secs: u64,
    force_immediately: bool,
) -> Result<Response, MixnetContractError> {
    ensure_is_owner(info.sender, deps.storage)?;

    if epochs_in_interval == 0 {
        return Err(MixnetContractError::EpochsInIntervalZero);
    }

    if epoch_duration_secs == 0 {
        return Err(MixnetContractError::EpochDurationZero);
    }

    let interval = storage::current_interval(deps.storage)?;
    if force_immediately || interval.is_current_interval_over(&env) {
        change_interval_config(
            deps.storage,
            env.block.height,
            interval,
            epochs_in_interval,
            epoch_duration_secs,
        )
    } else {
        // changing interval config is only allowed if the epoch is currently not in the process of being advanced
        // (unless the force flag was used)
        ensure_epoch_in_progress_state(deps.storage)?;

        // push the interval event
        let interval_event = PendingIntervalEventKind::UpdateIntervalConfig {
            epochs_in_interval,
            epoch_duration_secs,
        };
        push_new_interval_event(deps.storage, &env, interval_event)?;
        let time_left = interval.secs_until_current_interval_end(&env);
        Ok(
            Response::new().add_event(new_pending_interval_config_update_event(
                epochs_in_interval,
                epoch_duration_secs,
                time_left,
            )),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::fixtures;
    use crate::support::tests::test_helpers::TestSetup;
    use cosmwasm_std::Addr;
    use mixnet_contract_common::pending_events::PendingEpochEventKind;

    fn push_n_dummy_epoch_actions(test: &mut TestSetup, n: usize) {
        // if you attempt to undelegate non-existent delegation,
        // it will return an empty response, but will not fail
        let env = test.env();
        for i in 0..n {
            let dummy_action =
                PendingEpochEventKind::new_undelegate(Addr::unchecked("foomp"), i as MixId);
            storage::push_new_epoch_event(test.deps_mut().storage, &env, dummy_action).unwrap();
        }
    }

    fn push_n_dummy_interval_actions(test: &mut TestSetup, n: usize) {
        // if you attempt to update cost parameters of an unbonded mixnode,
        // it will return an empty response, but will not fail
        let env = test.env();
        for i in 0..n {
            let dummy_action = PendingIntervalEventKind::ChangeMixCostParams {
                mix_id: i as MixId,
                new_costs: fixtures::mix_node_cost_params_fixture(),
            };
            storage::push_new_interval_event(test.deps_mut().storage, &env, dummy_action).unwrap();
        }
    }

    #[cfg(test)]
    mod performing_pending_epoch_actions {
        use super::*;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use cosmwasm_std::{coin, coins, BankMsg, Empty, SubMsg};
        use mixnet_contract_common::events::{
            new_active_set_update_event, new_delegation_on_unbonded_node_event,
            new_undelegation_event,
        };

        #[test]
        fn without_limit_executes_all_actions() {
            let mut test = TestSetup::new();
            assert_eq!(
                0,
                storage::LAST_PROCESSED_EPOCH_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );

            let env = test.env();
            // no events are pending, nothing should get done
            let (res, executed) =
                perform_pending_epoch_actions(test.deps_mut(), &env, None).unwrap();
            assert_eq!(res, Response::new());
            assert_eq!(executed, 0);
            assert_eq!(
                0,
                storage::LAST_PROCESSED_EPOCH_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );

            push_n_dummy_epoch_actions(&mut test, 42);
            let (res, executed) =
                perform_pending_epoch_actions(test.deps_mut(), &env, None).unwrap();
            // dummy actions don't emit any events
            assert_eq!(res, Response::new());
            assert_eq!(executed, 42);
            assert_eq!(
                42,
                storage::LAST_PROCESSED_EPOCH_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );

            push_n_dummy_epoch_actions(&mut test, 10);
            let (res, executed) =
                perform_pending_epoch_actions(test.deps_mut(), &env, None).unwrap();
            assert_eq!(res, Response::new());
            assert_eq!(executed, 10);
            assert_eq!(
                52,
                storage::LAST_PROCESSED_EPOCH_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );

            push_n_dummy_epoch_actions(&mut test, 1);
            let (res, executed) =
                perform_pending_epoch_actions(test.deps_mut(), &env, None).unwrap();
            assert_eq!(res, Response::new());
            assert_eq!(executed, 1);
            assert_eq!(
                53,
                storage::LAST_PROCESSED_EPOCH_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );

            push_n_dummy_epoch_actions(&mut test, 10);
            let action_with_event = PendingEpochEventKind::UpdateActiveSetSize { new_size: 50 };
            storage::push_new_epoch_event(test.deps_mut().storage, &env, action_with_event)
                .unwrap();
            push_n_dummy_epoch_actions(&mut test, 10);
            let (res, executed) =
                perform_pending_epoch_actions(test.deps_mut(), &env, None).unwrap();
            assert_eq!(
                res,
                Response::new().add_event(new_active_set_update_event(env.block.height, 50))
            );
            assert_eq!(executed, 21);
            assert_eq!(
                74,
                storage::LAST_PROCESSED_EPOCH_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );
        }

        #[test]
        fn catches_all_events_and_messages_from_executed_actions() {
            let mut test = TestSetup::new();

            let env = test.env();
            let legit_mix = test.add_dummy_mixnode("mix-owner", None);
            let delegator = Addr::unchecked("delegator");
            let amount = 123_456_789u128;
            test.add_immediate_delegation(delegator.as_str(), amount, legit_mix);

            let mut expected_events = Vec::new();
            let mut expected_messages: Vec<SubMsg<Empty>> = Vec::new();

            // delegate to node that doesn't exist,
            // we expect to receive BankMsg with tokens being returned,
            // and event regarding delegation
            let non_existent_delegation = PendingEpochEventKind::new_delegate(
                Addr::unchecked("foomp"),
                123,
                coin(123, TEST_COIN_DENOM),
            );
            storage::push_new_epoch_event(test.deps_mut().storage, &env, non_existent_delegation)
                .unwrap();
            expected_events.push(new_delegation_on_unbonded_node_event(
                &Addr::unchecked("foomp"),
                123,
            ));
            expected_messages.push(SubMsg::new(BankMsg::Send {
                to_address: "foomp".to_string(),
                amount: coins(123, TEST_COIN_DENOM),
            }));

            // updating active set should only emit events and no cosmos messages
            let action_with_event = PendingEpochEventKind::UpdateActiveSetSize { new_size: 50 };
            storage::push_new_epoch_event(test.deps_mut().storage, &env, action_with_event)
                .unwrap();
            expected_events.push(new_active_set_update_event(env.block.height, 50));

            // undelegation just returns tokens and emits event
            let legit_undelegate =
                PendingEpochEventKind::new_undelegate(delegator.clone(), legit_mix);
            storage::push_new_epoch_event(test.deps_mut().storage, &env, legit_undelegate).unwrap();
            expected_events.push(new_undelegation_event(
                env.block.height,
                &delegator,
                legit_mix,
            ));
            expected_messages.push(SubMsg::new(BankMsg::Send {
                to_address: delegator.into_string(),
                amount: coins(amount, TEST_COIN_DENOM),
            }));

            let (res, executed) =
                perform_pending_epoch_actions(test.deps_mut(), &env, None).unwrap();
            let mut expected = Response::new().add_events(expected_events);
            expected.messages = expected_messages;
            assert_eq!(res, expected);
            assert_eq!(executed, 3);
            assert_eq!(
                3,
                storage::LAST_PROCESSED_EPOCH_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );
        }

        #[test]
        fn respects_limit() {
            let mut test = TestSetup::new();

            let env = test.env();

            push_n_dummy_epoch_actions(&mut test, 20);

            // no events are pending, nothing should get done
            let (_, executed) =
                perform_pending_epoch_actions(test.deps_mut(), &env, Some(0)).unwrap();
            assert_eq!(executed, 0);
            assert_eq!(
                0,
                storage::LAST_PROCESSED_EPOCH_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );

            let (_, executed) =
                perform_pending_epoch_actions(test.deps_mut(), &env, Some(10)).unwrap();
            assert_eq!(executed, 10);
            assert_eq!(
                10,
                storage::LAST_PROCESSED_EPOCH_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );

            let (_, executed) =
                perform_pending_epoch_actions(test.deps_mut(), &env, Some(10)).unwrap();
            assert_eq!(executed, 10);
            assert_eq!(
                20,
                storage::LAST_PROCESSED_EPOCH_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );

            push_n_dummy_epoch_actions(&mut test, 20);
            let (_, executed) =
                perform_pending_epoch_actions(test.deps_mut(), &env, Some(100)).unwrap();
            assert_eq!(executed, 20);
            assert_eq!(
                40,
                storage::LAST_PROCESSED_EPOCH_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );
        }
    }

    #[cfg(test)]
    mod performing_pending_interval_actions {
        use super::*;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use cosmwasm_std::{coin, Empty, SubMsg};
        use mixnet_contract_common::events::{
            new_interval_config_update_event, new_mixnode_cost_params_update_event,
            new_rewarding_params_update_event,
        };
        use mixnet_contract_common::mixnode::MixNodeCostParams;
        use mixnet_contract_common::reward_params::IntervalRewardingParamsUpdate;
        use mixnet_contract_common::Percent;

        #[test]
        fn without_limit_executes_all_actions() {
            let mut test = TestSetup::new();
            assert_eq!(
                0,
                storage::LAST_PROCESSED_INTERVAL_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );

            let env = test.env();
            // no events are pending, nothing should get done
            let (res, executed) =
                perform_pending_interval_actions(test.deps_mut(), &env, None).unwrap();
            assert_eq!(res, Response::new());
            assert_eq!(executed, 0);
            assert_eq!(
                0,
                storage::LAST_PROCESSED_INTERVAL_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );

            push_n_dummy_interval_actions(&mut test, 42);
            let (res, executed) =
                perform_pending_interval_actions(test.deps_mut(), &env, None).unwrap();
            // dummy actions don't emit any events
            assert_eq!(res, Response::new());
            assert_eq!(executed, 42);
            assert_eq!(
                42,
                storage::LAST_PROCESSED_INTERVAL_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );

            push_n_dummy_interval_actions(&mut test, 10);
            let (res, executed) =
                perform_pending_interval_actions(test.deps_mut(), &env, None).unwrap();
            assert_eq!(res, Response::new());
            assert_eq!(executed, 10);
            assert_eq!(
                52,
                storage::LAST_PROCESSED_INTERVAL_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );

            push_n_dummy_interval_actions(&mut test, 1);
            let (res, executed) =
                perform_pending_interval_actions(test.deps_mut(), &env, None).unwrap();
            assert_eq!(res, Response::new());
            assert_eq!(executed, 1);
            assert_eq!(
                53,
                storage::LAST_PROCESSED_INTERVAL_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );

            push_n_dummy_interval_actions(&mut test, 10);
            let update = IntervalRewardingParamsUpdate {
                rewarded_set_size: Some(500),
                ..Default::default()
            };
            let action_with_event = PendingIntervalEventKind::UpdateRewardingParams { update };
            storage::push_new_interval_event(test.deps_mut().storage, &env, action_with_event)
                .unwrap();
            push_n_dummy_interval_actions(&mut test, 10);
            let (res, executed) =
                perform_pending_interval_actions(test.deps_mut(), &env, None).unwrap();
            let updated_params = test.rewarding_params().interval;
            assert_eq!(
                res,
                Response::new().add_event(new_rewarding_params_update_event(
                    env.block.height,
                    update,
                    updated_params
                ))
            );
            assert_eq!(executed, 21);
            assert_eq!(
                74,
                storage::LAST_PROCESSED_INTERVAL_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );
        }

        #[test]
        fn catches_all_events_and_messages_from_executed_actions() {
            let mut test = TestSetup::new();
            let env = test.env();

            let mut expected_events = Vec::new();
            let expected_messages: Vec<SubMsg<Empty>> = Vec::new();

            let legit_mix = test.add_dummy_mixnode("mix-owner", None);
            let new_costs = MixNodeCostParams {
                profit_margin_percent: Percent::from_percentage_value(12).unwrap(),
                interval_operating_cost: coin(123_000, TEST_COIN_DENOM),
            };
            let cost_change = PendingIntervalEventKind::ChangeMixCostParams {
                mix_id: legit_mix,
                new_costs: new_costs.clone(),
            };

            storage::push_new_interval_event(test.deps_mut().storage, &env, cost_change).unwrap();
            expected_events.push(new_mixnode_cost_params_update_event(
                env.block.height,
                legit_mix,
                &new_costs,
            ));

            let update = IntervalRewardingParamsUpdate {
                rewarded_set_size: Some(500),
                ..Default::default()
            };
            let change_params = PendingIntervalEventKind::UpdateRewardingParams { update };
            storage::push_new_interval_event(test.deps_mut().storage, &env, change_params).unwrap();
            let interval = test.current_interval();
            let mut expected_updated = test.rewarding_params();
            expected_updated
                .try_apply_updates(update, interval.epochs_in_interval())
                .unwrap();
            expected_events.push(new_rewarding_params_update_event(
                env.block.height,
                update,
                expected_updated.interval,
            ));

            let change_interval = PendingIntervalEventKind::UpdateIntervalConfig {
                epochs_in_interval: 123,
                epoch_duration_secs: 1000,
            };
            let mut expected_updated2 = expected_updated;
            expected_updated2.apply_epochs_in_interval_change(123);
            storage::push_new_interval_event(test.deps_mut().storage, &env, change_interval)
                .unwrap();
            expected_events.push(new_interval_config_update_event(
                env.block.height,
                123,
                1000,
                expected_updated2.interval,
            ));

            let (res, executed) =
                perform_pending_interval_actions(test.deps_mut(), &env, None).unwrap();
            let mut expected = Response::new().add_events(expected_events);
            expected.messages = expected_messages;
            assert_eq!(res, expected);
            assert_eq!(executed, 3);
            assert_eq!(
                3,
                storage::LAST_PROCESSED_INTERVAL_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );
        }

        #[test]
        fn respects_limit() {
            let mut test = TestSetup::new();

            let env = test.env();

            push_n_dummy_interval_actions(&mut test, 20);

            // no events are pending, nothing should get done
            let (_, executed) =
                perform_pending_interval_actions(test.deps_mut(), &env, Some(0)).unwrap();
            assert_eq!(executed, 0);
            assert_eq!(
                0,
                storage::LAST_PROCESSED_INTERVAL_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );

            let (_, executed) =
                perform_pending_interval_actions(test.deps_mut(), &env, Some(10)).unwrap();
            assert_eq!(executed, 10);
            assert_eq!(
                10,
                storage::LAST_PROCESSED_INTERVAL_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );

            let (_, executed) =
                perform_pending_interval_actions(test.deps_mut(), &env, Some(10)).unwrap();
            assert_eq!(executed, 10);
            assert_eq!(
                20,
                storage::LAST_PROCESSED_INTERVAL_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );

            push_n_dummy_interval_actions(&mut test, 20);
            let (_, executed) =
                perform_pending_interval_actions(test.deps_mut(), &env, Some(100)).unwrap();
            assert_eq!(executed, 20);
            assert_eq!(
                40,
                storage::LAST_PROCESSED_INTERVAL_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );
        }
    }

    #[cfg(test)]
    mod beginning_epoch_transition {
        use super::*;
        use cosmwasm_std::testing::mock_info;

        #[test]
        fn returns_error_if_epoch_is_in_progress() {
            let mut test = TestSetup::new();
            let env = test.env();
            let rewarding_validator = test.rewarding_validator();

            let res = try_begin_epoch_transition(test.deps_mut(), env, rewarding_validator);
            assert!(matches!(
                res,
                Err(MixnetContractError::EpochInProgress { .. })
            ))
        }

        #[test]
        fn can_only_be_performed_if_in_progress_state() {
            let bad_states = vec![
                EpochState::Rewarding {
                    last_rewarded: 0,
                    final_node_id: 0,
                },
                EpochState::ReconcilingEvents,
                EpochState::AdvancingEpoch,
            ];

            for bad_state in bad_states {
                let mut test = TestSetup::new();
                let rewarding_validator = test.rewarding_validator();

                let mut status = EpochStatus::new(test.rewarding_validator().sender);
                status.state = bad_state;
                storage::save_current_epoch_status(test.deps_mut().storage, &status).unwrap();

                test.skip_to_current_epoch_end();
                let env = test.env();

                let res = try_begin_epoch_transition(test.deps_mut(), env, rewarding_validator);
                assert_eq!(
                    res,
                    Err(MixnetContractError::EpochAdvancementInProgress {
                        current_state: bad_state
                    })
                );
            }
        }

        #[test]
        fn returns_error_if_not_performed_by_the_rewarding_validator() {
            let mut test = TestSetup::new();
            let env = test.env();

            test.skip_to_current_epoch_end();

            let random = mock_info("alice", &[]);
            let owner = test.owner();

            let res = try_begin_epoch_transition(test.deps_mut(), env.clone(), random);
            assert!(matches!(res, Err(MixnetContractError::Unauthorized)));

            let res = try_begin_epoch_transition(test.deps_mut(), env, owner);
            assert!(matches!(res, Err(MixnetContractError::Unauthorized)));
        }

        #[test]
        fn returns_error_if_epoch_is_already_being_advanced() {
            let mut test = TestSetup::new();
            let rewarding_validator = test.rewarding_validator();

            test.skip_to_current_epoch_end();
            let env = test.env();

            try_begin_epoch_transition(test.deps_mut(), env.clone(), rewarding_validator.clone())
                .unwrap();

            let res = try_begin_epoch_transition(test.deps_mut(), env, rewarding_validator);
            assert!(matches!(
                res,
                Err(MixnetContractError::EpochAdvancementInProgress { .. })
            ));
        }

        #[test]
        fn epoch_state_is_correctly_updated_for_empty_rewarded_set() {
            let mut test = TestSetup::new();
            let rewarding_validator = test.rewarding_validator();

            test.skip_to_current_epoch_end();
            let env = test.env();

            try_begin_epoch_transition(test.deps_mut(), env, rewarding_validator).unwrap();

            let expected = EpochStatus {
                being_advanced_by: test.rewarding_validator().sender,
                state: EpochState::ReconcilingEvents,
            };
            assert_eq!(
                expected,
                storage::current_epoch_status(test.deps().storage).unwrap()
            )
        }

        #[test]
        fn epoch_state_is_correctly_updated_for_nonempty_rewarded_set() {
            let mut test = TestSetup::new();
            let rewarding_validator = test.rewarding_validator();

            test.force_change_rewarded_set(vec![1, 2, 3, 4, 5]);
            test.skip_to_current_epoch_end();
            let env = test.env();

            try_begin_epoch_transition(test.deps_mut(), env, rewarding_validator).unwrap();

            let expected = EpochStatus {
                being_advanced_by: test.rewarding_validator().sender,
                state: EpochState::Rewarding {
                    last_rewarded: 0,
                    final_node_id: 5,
                },
            };
            assert_eq!(
                expected,
                storage::current_epoch_status(test.deps().storage).unwrap()
            )
        }
    }

    #[cfg(test)]
    mod reconciling_epoch_events {
        use super::*;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use cosmwasm_std::testing::mock_info;
        use cosmwasm_std::{coin, coins, BankMsg, Empty, SubMsg};
        use mixnet_contract_common::events::{
            new_delegation_on_unbonded_node_event, new_rewarding_params_update_event,
        };
        use mixnet_contract_common::reward_params::IntervalRewardingParamsUpdate;

        #[test]
        fn can_only_be_performed_if_in_reconciling_state() {
            let bad_states = vec![
                EpochState::InProgress,
                EpochState::Rewarding {
                    last_rewarded: 0,
                    final_node_id: 0,
                },
                EpochState::AdvancingEpoch,
            ];

            for bad_state in bad_states {
                let mut test = TestSetup::new();
                let rewarding_validator = test.rewarding_validator();

                let mut status = EpochStatus::new(test.rewarding_validator().sender);
                status.state = bad_state;
                storage::save_current_epoch_status(test.deps_mut().storage, &status).unwrap();

                test.skip_to_current_epoch_end();
                let env = test.env();

                let res =
                    try_reconcile_epoch_events(test.deps_mut(), env, rewarding_validator, None);
                assert_eq!(
                    res,
                    Err(MixnetContractError::EpochNotInEventReconciliationState {
                        current_state: bad_state
                    })
                );
            }
        }

        #[test]
        fn epoch_state_is_correctly_updated_if_there_are_no_events() {
            let mut test = TestSetup::new();
            let rewarding_validator = test.rewarding_validator();

            test.skip_to_current_epoch_end();
            test.set_epoch_reconciliation_state();
            let env = test.env();

            try_reconcile_epoch_events(test.deps_mut(), env, rewarding_validator, None).unwrap();

            let expected = EpochStatus {
                being_advanced_by: test.rewarding_validator().sender,
                state: EpochState::AdvancingEpoch,
            };
            assert_eq!(
                expected,
                storage::current_epoch_status(test.deps().storage).unwrap()
            )
        }

        #[test]
        fn epoch_state_is_not_updated_if_some_events_are_cleared() {
            let mut test = TestSetup::new();
            let rewarding_validator = test.rewarding_validator();

            test.skip_to_current_epoch_end();
            let env = test.env();

            push_n_dummy_epoch_actions(&mut test, 10);
            push_n_dummy_interval_actions(&mut test, 10);
            test.set_epoch_reconciliation_state();

            try_reconcile_epoch_events(test.deps_mut(), env, rewarding_validator, Some(5)).unwrap();

            let expected = EpochStatus {
                being_advanced_by: test.rewarding_validator().sender,
                state: EpochState::ReconcilingEvents,
            };
            assert_eq!(
                expected,
                storage::current_epoch_status(test.deps().storage).unwrap()
            )
        }

        #[test]
        fn epoch_state_is_correctly_updated_if_even_with_leftover_interval_events_if_interval_is_not_over(
        ) {
            let mut test = TestSetup::new();
            let rewarding_validator = test.rewarding_validator();

            test.skip_to_current_epoch_end();
            let env = test.env();

            push_n_dummy_epoch_actions(&mut test, 10);
            push_n_dummy_interval_actions(&mut test, 10);
            test.set_epoch_reconciliation_state();

            try_reconcile_epoch_events(test.deps_mut(), env, rewarding_validator, None).unwrap();

            let expected = EpochStatus {
                being_advanced_by: test.rewarding_validator().sender,
                state: EpochState::AdvancingEpoch,
            };
            assert_eq!(
                expected,
                storage::current_epoch_status(test.deps().storage).unwrap()
            )
        }

        #[test]
        fn epoch_state_is_correctly_updated_if_all_events_are_cleared() {
            let mut test = TestSetup::new();
            let rewarding_validator = test.rewarding_validator();

            test.skip_to_current_interval_end();
            let env = test.env();

            push_n_dummy_epoch_actions(&mut test, 10);
            push_n_dummy_interval_actions(&mut test, 10);
            test.set_epoch_reconciliation_state();

            try_reconcile_epoch_events(test.deps_mut(), env, rewarding_validator, None).unwrap();

            let expected = EpochStatus {
                being_advanced_by: test.rewarding_validator().sender,
                state: EpochState::AdvancingEpoch,
            };
            assert_eq!(
                expected,
                storage::current_epoch_status(test.deps().storage).unwrap()
            )
        }

        #[test]
        fn returns_error_if_epoch_is_in_progress() {
            let mut test = TestSetup::new();
            let env = test.env();
            let rewarding_validator = test.rewarding_validator();

            test.set_epoch_reconciliation_state();
            let res = try_reconcile_epoch_events(test.deps_mut(), env, rewarding_validator, None);
            assert!(matches!(
                res,
                Err(MixnetContractError::EpochInProgress { .. })
            ))
        }

        #[test]
        fn returns_error_if_not_performed_by_the_rewarding_validator() {
            let mut test = TestSetup::new();
            let env = test.env();

            test.skip_to_current_epoch_end();
            test.set_epoch_reconciliation_state();

            let random = mock_info("alice", &[]);
            let owner = test.owner();

            let res = try_reconcile_epoch_events(test.deps_mut(), env.clone(), random, None);
            assert!(matches!(res, Err(MixnetContractError::Unauthorized)));

            let res = try_reconcile_epoch_events(test.deps_mut(), env, owner, None);
            assert!(matches!(res, Err(MixnetContractError::Unauthorized)));
        }

        #[test]
        fn only_clears_epoch_events_if_interval_is_in_progress() {
            let mut test = TestSetup::new();

            push_n_dummy_epoch_actions(&mut test, 10);
            push_n_dummy_interval_actions(&mut test, 10);
            test.skip_to_current_epoch_end();
            test.set_epoch_reconciliation_state();

            let env = test.env();
            let rewarding_validator = test.rewarding_validator();

            try_reconcile_epoch_events(test.deps_mut(), env, rewarding_validator, None).unwrap();

            let epoch_events = test.pending_epoch_events();
            let interval_events = test.pending_interval_events();
            assert!(epoch_events.is_empty());
            assert_eq!(interval_events.len(), 10);
        }

        #[test]
        fn clears_both_epoch_and_interval_events_if_interval_has_finished() {
            let mut test = TestSetup::new();

            push_n_dummy_epoch_actions(&mut test, 10);
            push_n_dummy_interval_actions(&mut test, 10);
            test.skip_to_current_interval_end();
            test.set_epoch_reconciliation_state();
            let rewarding_validator = test.rewarding_validator();

            let env = test.env();
            try_reconcile_epoch_events(test.deps_mut(), env, rewarding_validator, None).unwrap();

            let epoch_events = test.pending_epoch_events();
            let interval_events = test.pending_interval_events();
            assert!(epoch_events.is_empty());
            assert!(interval_events.is_empty());
        }

        #[test]
        fn with_limit_prioritises_epoch_events() {
            let mut test1 = TestSetup::new();
            let mut test2 = TestSetup::new();
            let mut test3 = TestSetup::new();
            let mut test4 = TestSetup::new();

            for test in [&mut test1, &mut test2, &mut test3, &mut test4].iter_mut() {
                push_n_dummy_epoch_actions(test, 10);
                push_n_dummy_interval_actions(test, 10);
                test.skip_to_current_interval_end();
            }

            let env = test1.env();
            // all test cases are using the same one
            let rewarding_validator = test1.rewarding_validator();

            test1.set_epoch_reconciliation_state();
            try_reconcile_epoch_events(
                test1.deps_mut(),
                env.clone(),
                rewarding_validator.clone(),
                Some(5),
            )
            .unwrap();
            let epoch_events = test1.pending_epoch_events();
            let interval_events = test1.pending_interval_events();
            assert_eq!(epoch_events.len(), 5);
            assert_eq!(interval_events.len(), 10);

            try_reconcile_epoch_events(
                test1.deps_mut(),
                env.clone(),
                rewarding_validator.clone(),
                Some(7),
            )
            .unwrap();
            let epoch_events = test1.pending_epoch_events();
            let interval_events = test1.pending_interval_events();
            assert!(epoch_events.is_empty());
            assert_eq!(interval_events.len(), 8);

            try_reconcile_epoch_events(
                test1.deps_mut(),
                env.clone(),
                rewarding_validator.clone(),
                Some(7),
            )
            .unwrap();
            let epoch_events = test1.pending_epoch_events();
            let interval_events = test1.pending_interval_events();
            assert!(epoch_events.is_empty());
            assert_eq!(interval_events.len(), 1);

            try_reconcile_epoch_events(
                test1.deps_mut(),
                env.clone(),
                rewarding_validator.clone(),
                Some(7),
            )
            .unwrap();
            let epoch_events = test1.pending_epoch_events();
            let interval_events = test1.pending_interval_events();
            assert!(epoch_events.is_empty());
            assert!(interval_events.is_empty());

            test2.set_epoch_reconciliation_state();
            try_reconcile_epoch_events(
                test2.deps_mut(),
                env.clone(),
                rewarding_validator.clone(),
                Some(15),
            )
            .unwrap();
            let epoch_events = test2.pending_epoch_events();
            let interval_events = test2.pending_interval_events();
            assert!(epoch_events.is_empty());
            assert_eq!(interval_events.len(), 5);

            try_reconcile_epoch_events(
                test2.deps_mut(),
                env.clone(),
                rewarding_validator.clone(),
                Some(15),
            )
            .unwrap();
            let epoch_events = test2.pending_epoch_events();
            let interval_events = test2.pending_interval_events();
            assert!(epoch_events.is_empty());
            assert!(interval_events.is_empty());

            test3.set_epoch_reconciliation_state();
            try_reconcile_epoch_events(
                test3.deps_mut(),
                env.clone(),
                rewarding_validator.clone(),
                Some(20),
            )
            .unwrap();
            let epoch_events = test3.pending_epoch_events();
            let interval_events = test3.pending_interval_events();
            assert!(epoch_events.is_empty());
            assert!(interval_events.is_empty());

            test4.set_epoch_reconciliation_state();
            try_reconcile_epoch_events(test4.deps_mut(), env, rewarding_validator, Some(100))
                .unwrap();
            let epoch_events = test4.pending_epoch_events();
            let interval_events = test4.pending_interval_events();
            assert!(epoch_events.is_empty());
            assert!(interval_events.is_empty());
        }

        #[test]
        fn catches_all_emitted_cosmos_events_and_messages() {
            let mut test = TestSetup::new();
            let env = test.env();

            let mut expected_events = vec![new_reconcile_pending_events()];
            let mut expected_messages: Vec<SubMsg<Empty>> = Vec::new();

            // epoch event
            let non_existent_delegation = PendingEpochEventKind::new_delegate(
                Addr::unchecked("foomp"),
                123,
                coin(123, TEST_COIN_DENOM),
            );
            storage::push_new_epoch_event(test.deps_mut().storage, &env, non_existent_delegation)
                .unwrap();
            expected_events.push(new_delegation_on_unbonded_node_event(
                &Addr::unchecked("foomp"),
                123,
            ));
            expected_messages.push(SubMsg::new(BankMsg::Send {
                to_address: "foomp".to_string(),
                amount: coins(123, TEST_COIN_DENOM),
            }));
            expected_events.push(new_pending_epoch_events_execution_event(1));

            // interval event
            let update = IntervalRewardingParamsUpdate {
                rewarded_set_size: Some(500),
                ..Default::default()
            };
            let change_params = PendingIntervalEventKind::UpdateRewardingParams { update };
            storage::push_new_interval_event(test.deps_mut().storage, &env, change_params).unwrap();
            let interval = test.current_interval();
            let mut expected_updated = test.rewarding_params();
            expected_updated
                .try_apply_updates(update, interval.epochs_in_interval())
                .unwrap();
            expected_events.push(new_rewarding_params_update_event(
                env.block.height,
                update,
                expected_updated.interval,
            ));
            expected_events.push(new_pending_interval_events_execution_event(1));

            test.skip_to_current_interval_end();
            test.set_epoch_reconciliation_state();
            let env = test.env();
            let rewarding_validator = test.rewarding_validator();

            let res = try_reconcile_epoch_events(test.deps_mut(), env, rewarding_validator, None)
                .unwrap();
            let mut expected = Response::new().add_events(expected_events);
            expected.messages = expected_messages;
            assert_eq!(res, expected);
            assert_eq!(
                1,
                storage::LAST_PROCESSED_EPOCH_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );
            assert_eq!(
                1,
                storage::LAST_PROCESSED_INTERVAL_EVENT
                    .load(test.deps().storage)
                    .unwrap()
            );
        }
    }

    #[test]
    fn updating_rewarded_set() {
        // the actual logic behind writing stuff to the storage has been tested in
        // different unit test
        let mut test = TestSetup::new();
        let current_active_set = test.rewarding_params().active_set_size;
        let current_rewarded_set = test.rewarding_params().rewarded_set_size;

        // active set size has to match the expectation
        let err = update_rewarded_set(
            test.deps_mut().storage,
            vec![1, 2, 3],
            current_active_set - 10,
        )
        .unwrap_err();
        assert_eq!(
            err,
            MixnetContractError::UnexpectedActiveSetSize {
                received: current_active_set - 10,
                expected: current_active_set,
            }
        );

        // number of nodes provided has to be equal or smaller than the current rewarded set size

        // fewer nodes
        let res = update_rewarded_set(test.deps_mut().storage, vec![1, 2, 3], current_active_set);
        assert!(res.is_ok());

        let exact_num = (1u32..)
            .take(current_rewarded_set as usize)
            .collect::<Vec<_>>();
        let res = update_rewarded_set(test.deps_mut().storage, exact_num, current_active_set);
        assert!(res.is_ok());

        // one more
        let too_many = (1u32..)
            .take((current_rewarded_set + 1) as usize)
            .collect::<Vec<_>>();
        let err =
            update_rewarded_set(test.deps_mut().storage, too_many, current_active_set).unwrap_err();
        assert_eq!(
            err,
            MixnetContractError::UnexpectedRewardedSetSize {
                received: current_rewarded_set + 1,
                expected: current_rewarded_set,
            }
        );

        // doesn't allow for duplicates
        let nodes_with_duplicate = vec![1, 2, 3, 4, 5, 1];
        let err = update_rewarded_set(
            test.deps_mut().storage,
            nodes_with_duplicate,
            current_active_set,
        )
        .unwrap_err();
        assert_eq!(
            err,
            MixnetContractError::DuplicateRewardedSetNode { mix_id: 1 }
        );
        let nodes_with_duplicate = vec![1, 2, 3, 5, 4, 5];
        let err = update_rewarded_set(
            test.deps_mut().storage,
            nodes_with_duplicate,
            current_active_set,
        )
        .unwrap_err();
        assert_eq!(
            err,
            MixnetContractError::DuplicateRewardedSetNode { mix_id: 5 }
        );
    }

    #[cfg(test)]
    mod advancing_epoch {
        use super::*;
        use crate::mixnodes::queries::query_mixnode_details;
        use crate::rewards::models::RewardPoolChange;
        use cosmwasm_std::testing::mock_info;
        use cosmwasm_std::{Decimal, Uint128};
        use mixnet_contract_common::reward_params::IntervalRewardingParamsUpdate;
        use mixnet_contract_common::{Layer, RewardedSetNodeStatus};

        #[test]
        fn can_only_be_performed_if_in_advancing_epoch_state() {
            let bad_states = vec![
                EpochState::InProgress,
                EpochState::Rewarding {
                    last_rewarded: 0,
                    final_node_id: 0,
                },
                EpochState::ReconcilingEvents,
            ];

            for bad_state in bad_states {
                let mut test = TestSetup::new();
                test.add_dummy_mixnode("1", Some(Uint128::new(100000000)));
                test.add_dummy_mixnode("2", Some(Uint128::new(100000000)));
                test.add_dummy_mixnode("3", Some(Uint128::new(100000000)));
                let current_active_set = test.rewarding_params().active_set_size;

                test.skip_to_current_epoch_end();

                let mut status = EpochStatus::new(test.rewarding_validator().sender);
                status.state = bad_state;
                storage::save_current_epoch_status(test.deps_mut().storage, &status).unwrap();

                let layer_assignments = vec![
                    LayerAssignment::new(1, Layer::One),
                    LayerAssignment::new(2, Layer::Two),
                    LayerAssignment::new(3, Layer::Three),
                ];

                let env = test.env();
                let sender = test.rewarding_validator();
                let res = try_advance_epoch(
                    test.deps_mut(),
                    env,
                    sender,
                    layer_assignments,
                    current_active_set,
                );
                assert_eq!(
                    res,
                    Err(MixnetContractError::EpochNotInAdvancementState {
                        current_state: bad_state
                    })
                );
            }
        }

        #[test]
        fn epoch_state_is_correctly_updated() {
            let mut test = TestSetup::new();
            test.add_dummy_mixnode("1", Some(Uint128::new(100000000)));
            test.add_dummy_mixnode("2", Some(Uint128::new(100000000)));
            test.add_dummy_mixnode("3", Some(Uint128::new(100000000)));
            let current_active_set = test.rewarding_params().active_set_size;

            test.skip_to_current_epoch_end();
            test.set_epoch_advancement_state();

            let layer_assignments = vec![
                LayerAssignment::new(1, Layer::One),
                LayerAssignment::new(2, Layer::Two),
                LayerAssignment::new(3, Layer::Three),
            ];

            let env = test.env();
            let sender = test.rewarding_validator();
            try_advance_epoch(
                test.deps_mut(),
                env,
                sender,
                layer_assignments,
                current_active_set,
            )
            .unwrap();

            let expected = EpochStatus {
                being_advanced_by: test.rewarding_validator().sender,
                state: EpochState::InProgress,
            };
            assert_eq!(
                expected,
                storage::current_epoch_status(test.deps().storage).unwrap()
            )
        }

        #[test]
        fn can_only_be_performed_by_specified_rewarding_validator() {
            let mut test = TestSetup::new();
            test.add_dummy_mixnode("1", Some(Uint128::new(100000000)));
            test.add_dummy_mixnode("2", Some(Uint128::new(100000000)));
            test.add_dummy_mixnode("3", Some(Uint128::new(100000000)));
            let current_active_set = test.rewarding_params().active_set_size;
            let some_sender = mock_info("foomper", &[]);

            test.skip_to_current_epoch_end();
            test.set_epoch_advancement_state();

            let layer_assignments = vec![
                LayerAssignment::new(1, Layer::One),
                LayerAssignment::new(2, Layer::Two),
                LayerAssignment::new(3, Layer::Three),
            ];

            let env = test.env();
            let res = try_advance_epoch(
                test.deps_mut(),
                env,
                some_sender,
                layer_assignments.clone(),
                current_active_set,
            );
            assert_eq!(res, Err(MixnetContractError::Unauthorized));

            // good address (sanity check)
            let env = test.env();
            let sender = test.rewarding_validator();
            let res = try_advance_epoch(
                test.deps_mut(),
                env,
                sender,
                layer_assignments,
                current_active_set,
            );
            assert!(res.is_ok())
        }

        #[test]
        fn can_only_be_performed_if_epoch_is_over() {
            let mut test = TestSetup::new();
            test.set_epoch_advancement_state();

            let current_active_set = test.rewarding_params().active_set_size;

            test.add_dummy_mixnode("1", Some(Uint128::new(100000000)));
            test.add_dummy_mixnode("2", Some(Uint128::new(100000000)));
            test.add_dummy_mixnode("3", Some(Uint128::new(100000000)));

            let layer_assignments = vec![
                LayerAssignment::new(1, Layer::One),
                LayerAssignment::new(2, Layer::Two),
                LayerAssignment::new(3, Layer::Three),
            ];

            let env = test.env();
            let sender = test.rewarding_validator();
            let res = try_advance_epoch(
                test.deps_mut(),
                env,
                sender.clone(),
                layer_assignments.clone(),
                current_active_set,
            );
            assert!(matches!(
                res,
                Err(MixnetContractError::EpochInProgress { .. })
            ));

            let mixnode_1 = query_mixnode_details(test.deps.as_ref(), 1).unwrap();
            assert_eq!(
                mixnode_1.mixnode_details.unwrap().bond_information.layer,
                Layer::One
            );

            let mixnode_1 = query_mixnode_details(test.deps.as_ref(), 2).unwrap();
            assert_eq!(
                mixnode_1.mixnode_details.unwrap().bond_information.layer,
                Layer::Two
            );

            let mixnode_1 = query_mixnode_details(test.deps.as_ref(), 3).unwrap();
            assert_eq!(
                mixnode_1.mixnode_details.unwrap().bond_information.layer,
                Layer::Three
            );

            // sanity check
            test.skip_to_current_epoch_end();

            let env = test.env();
            let res = try_advance_epoch(
                test.deps_mut(),
                env,
                sender,
                layer_assignments,
                current_active_set,
            );
            assert!(res.is_ok())
        }

        #[test]
        fn if_interval_is_over_applies_reward_pool_changes() {
            let mut test = TestSetup::new();
            test.set_epoch_advancement_state();

            let current_active_set = test.rewarding_params().active_set_size;

            test.add_dummy_mixnode("1", Some(Uint128::new(100000000)));
            test.add_dummy_mixnode("2", Some(Uint128::new(100000000)));
            test.add_dummy_mixnode("3", Some(Uint128::new(100000000)));

            let start_params = test.rewarding_params();

            let pool_update = Decimal::from_atomics(100_000_000u32, 0).unwrap();
            // push some changes
            rewards_storage::PENDING_REWARD_POOL_CHANGE
                .save(
                    test.deps_mut().storage,
                    &RewardPoolChange {
                        removed: pool_update,
                        added: Default::default(),
                    },
                )
                .unwrap();

            let layer_assignments = vec![
                LayerAssignment::new(1, Layer::One),
                LayerAssignment::new(2, Layer::Two),
                LayerAssignment::new(3, Layer::Three),
            ];

            // end of epoch - nothing has happened
            let sender = test.rewarding_validator();
            test.skip_to_current_epoch_end();

            let env = test.env();
            try_advance_epoch(
                test.deps_mut(),
                env,
                sender,
                layer_assignments.clone(),
                current_active_set,
            )
            .unwrap();

            let params = test.rewarding_params();
            let pool_change = rewards_storage::PENDING_REWARD_POOL_CHANGE
                .load(test.deps().storage)
                .unwrap();
            assert_eq!(params, start_params);
            assert_eq!(pool_change.removed, pool_update);

            let sender = test.rewarding_validator();
            test.skip_to_current_interval_end();
            test.set_epoch_advancement_state();

            let env = test.env();
            try_advance_epoch(
                test.deps_mut(),
                env,
                sender,
                layer_assignments,
                current_active_set,
            )
            .unwrap();

            let epochs_in_interval = test.current_interval().epochs_in_interval();
            let update = IntervalRewardingParamsUpdate {
                reward_pool: Some(start_params.interval.reward_pool - pool_update),
                staking_supply: Some(start_params.interval.staking_supply + pool_update),
                ..Default::default()
            };
            let mut expected = start_params;
            expected
                .try_apply_updates(update, epochs_in_interval)
                .unwrap();

            let params = test.rewarding_params();
            let pool_change = rewards_storage::PENDING_REWARD_POOL_CHANGE
                .load(test.deps().storage)
                .unwrap();
            assert_eq!(params, expected);
            assert_eq!(pool_change.removed, Decimal::zero());
        }

        #[test]
        fn updates_rewarded_set_and_interval_data() {
            let mut test = TestSetup::new();
            test.set_epoch_advancement_state();

            let current_active_set = test.rewarding_params().active_set_size;

            test.add_dummy_mixnode("1", Some(Uint128::new(100000000)));
            test.add_dummy_mixnode("2", Some(Uint128::new(100000000)));
            test.add_dummy_mixnode("3", Some(Uint128::new(100000000)));

            let interval_pre = test.current_interval();
            let rewarded_set_pre = test.rewarded_set();
            assert!(rewarded_set_pre.is_empty());

            let layer_assignments = vec![
                LayerAssignment::new(1, Layer::One),
                LayerAssignment::new(2, Layer::Two),
                LayerAssignment::new(3, Layer::Three),
            ];

            let sender = test.rewarding_validator();
            test.skip_to_current_interval_end();
            let env = test.env();
            try_advance_epoch(
                test.deps_mut(),
                env,
                sender,
                layer_assignments,
                current_active_set,
            )
            .unwrap();

            let interval_post = test.current_interval();
            let rewarded_set = test.rewarded_set();

            let expected_id = interval_pre.current_epoch_absolute_id() + 1;
            assert_eq!(interval_post.current_epoch_absolute_id(), expected_id);
            assert_eq!(
                rewarded_set,
                vec![
                    (1, RewardedSetNodeStatus::Active),
                    (2, RewardedSetNodeStatus::Active),
                    (3, RewardedSetNodeStatus::Active)
                ]
            );
        }
    }

    #[cfg(test)]
    mod updating_interval_config {
        use super::*;
        use cosmwasm_std::testing::mock_info;
        use cosmwasm_std::Decimal;
        use std::time::Duration;

        #[test]
        fn cant_be_performed_if_epoch_transition_is_in_progress_unless_forced() {
            let bad_states = vec![
                EpochState::Rewarding {
                    last_rewarded: 0,
                    final_node_id: 0,
                },
                EpochState::ReconcilingEvents,
                EpochState::AdvancingEpoch,
            ];

            for bad_state in bad_states {
                let mut test = TestSetup::new();
                let owner = test.owner();
                let env = test.env();

                let mut status = EpochStatus::new(test.rewarding_validator().sender);
                status.state = bad_state;

                storage::save_current_epoch_status(test.deps_mut().storage, &status).unwrap();

                let res = try_update_interval_config(
                    test.deps_mut(),
                    env.clone(),
                    owner.clone(),
                    100,
                    1000,
                    false,
                );
                assert!(matches!(
                    res,
                    Err(MixnetContractError::EpochAdvancementInProgress { .. })
                ));

                let res_forced = try_update_interval_config(
                    test.deps_mut(),
                    env.clone(),
                    owner,
                    100,
                    1000,
                    true,
                );
                assert!(res_forced.is_ok())
            }
        }

        #[test]
        fn can_only_be_done_by_contract_owner() {
            let mut test = TestSetup::new();

            let rewarding_validator = test.rewarding_validator();
            let owner = test.owner();
            let random = mock_info("random-guy", &[]);

            let env = test.env();
            let res = try_update_interval_config(
                test.deps_mut(),
                env.clone(),
                rewarding_validator,
                100,
                1000,
                false,
            );
            assert_eq!(res, Err(MixnetContractError::Unauthorized));

            let res =
                try_update_interval_config(test.deps_mut(), env.clone(), random, 100, 1000, false);
            assert_eq!(res, Err(MixnetContractError::Unauthorized));

            let res = try_update_interval_config(test.deps_mut(), env, owner, 100, 1000, false);
            assert!(res.is_ok())
        }

        #[test]
        fn if_interval_is_finished_change_happens_immediately() {
            let mut test = TestSetup::new();
            let owner = test.owner();

            let old = test.rewarding_params();
            test.skip_to_current_interval_end();

            let env = test.env();
            let res =
                try_update_interval_config(test.deps_mut(), env, owner.clone(), 100, 1000, false);
            assert!(res.is_ok());
            let new = test.rewarding_params();
            let interval = test.current_interval();
            assert_ne!(old, new);
            assert_eq!(interval.epochs_in_interval(), 100);

            // sanity check for "normal" case
            let mut test = TestSetup::new();
            let env = test.env();
            let res = try_update_interval_config(test.deps_mut(), env, owner, 100, 1000, false);
            assert!(res.is_ok());
            let new = test.rewarding_params();
            assert_eq!(old, new);
        }

        #[test]
        fn if_update_is_forced_it_happens_immediately() {
            let mut test = TestSetup::new();
            let owner = test.owner();

            let old = test.rewarding_params();
            let env = test.env();
            let res = try_update_interval_config(test.deps_mut(), env, owner, 100, 1000, true);
            assert!(res.is_ok());
            let new = test.rewarding_params();
            let interval = test.current_interval();
            assert_ne!(old, new);
            assert_eq!(interval.epochs_in_interval(), 100);
        }

        #[test]
        fn without_forcing_it_change_happens_upon_clearing_interval_events() {
            let mut test = TestSetup::new();
            let owner = test.owner();

            let old = test.rewarding_params();
            let env = test.env();
            let res = try_update_interval_config(test.deps_mut(), env, owner, 100, 1000, false);
            assert!(res.is_ok());
            let new = test.rewarding_params();
            assert_eq!(old, new);

            // make sure it's actually saved to pending events
            let events = test.pending_interval_events();
            assert!(matches!(events[0].kind,
                PendingIntervalEventKind::UpdateIntervalConfig { epochs_in_interval, epoch_duration_secs } if epochs_in_interval == 100 && epoch_duration_secs == 1000
            ));

            test.execute_all_pending_events();
            let new = test.rewarding_params();
            let interval = test.current_interval();
            assert_ne!(old, new);
            assert_eq!(interval.epochs_in_interval(), 100);
        }

        #[test]
        fn upon_update_fields_are_recomputed_accordingly() {
            let mut test = TestSetup::new();
            let owner = test.owner();
            let two = Decimal::from_atomics(2u32, 0).unwrap();

            let params_before = test.rewarding_params();

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
            let env = test.env();
            try_update_interval_config(
                test.deps_mut(),
                env,
                owner,
                interval_before.epochs_in_interval() / 2,
                1234,
                true,
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
}

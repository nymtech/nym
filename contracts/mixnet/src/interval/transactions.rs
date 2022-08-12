// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::interval::helpers::change_interval_config;
use crate::interval::pending_events::ContractExecutableEvent;
use crate::interval::storage::push_new_interval_event;
use crate::rewards;
use crate::rewards::storage as rewards_storage;
use crate::support::helpers::{ensure_is_authorized, ensure_is_owner};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_advance_epoch_event, new_pending_epoch_events_execution_event,
    new_pending_interval_config_update_event, new_pending_interval_events_execution_event,
    new_reconcile_pending_events,
};
use mixnet_contract_common::pending_events::PendingIntervalEventData;
use mixnet_contract_common::NodeId;
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
    mut limit: Option<u32>,
) -> Result<Response, MixnetContractError> {
    let mut response = Response::new().add_event(new_reconcile_pending_events());

    // there's no need for authorization, as anyone willing to pay the fees should be allowed to reconcile
    // contract events ASSUMING the corresponding epoch/interval has finished
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

    Ok(response)
}

fn update_rewarded_set(
    storage: &mut dyn Storage,
    new_rewarded_set: Vec<NodeId>,
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
            return Err(MixnetContractError::DuplicateRewardedSetNode { node_id: *node_id });
        }
    }

    Ok(storage::update_rewarded_set(
        storage,
        expected_active_set_size,
        new_rewarded_set,
    )?)
}

pub fn try_advance_epoch(
    mut deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    new_rewarded_set: Vec<NodeId>,
    expected_active_set_size: u32,
) -> Result<Response, MixnetContractError> {
    // Only rewarding validator can attempt to advance epoch
    ensure_is_authorized(info.sender, deps.storage)?;

    let mut response = Response::new();

    // we must make sure that we roll into new epoch / interval with up to date state
    // with no pending actions (like somebody wanting to update their profit margin)
    let current_interval = storage::current_interval(deps.storage)?;
    if !current_interval.is_current_epoch_over(&env) {
        return Err(MixnetContractError::EpochInProgress {
            current_block_time: env.block.time.seconds(),
            epoch_start: current_interval.current_epoch_start_unix_timestamp(),
            epoch_end: current_interval.current_epoch_end_unix_timestamp(),
        });
    } else {
        let (mut sub_response, executed) =
            perform_pending_epoch_actions(deps.branch(), &env, None)?;
        response.messages.append(&mut sub_response.messages);
        response.attributes.append(&mut sub_response.attributes);
        response.events.append(&mut sub_response.events);
        response
            .events
            .push(new_pending_epoch_events_execution_event(executed));
    }

    // first clear epoch events queue and then touch the interval actions
    if current_interval.is_current_interval_over(&env) {
        // the interval has finished -> we can change things such as the profit margin
        let (mut sub_response, executed) =
            perform_pending_interval_actions(deps.branch(), &env, None)?;
        response.messages.append(&mut sub_response.messages);
        response.attributes.append(&mut sub_response.attributes);
        response.events.append(&mut sub_response.events);
        response
            .events
            .push(new_pending_interval_events_execution_event(executed));

        // this one is a very important one!
        rewards::helpers::apply_reward_pool_changes(deps.storage)?;
    }

    let updated_interval = current_interval.advance_epoch();
    let num_nodes = new_rewarded_set.len();

    // finally save updated interval and the rewarded set
    storage::save_interval(deps.storage, &updated_interval)?;
    update_rewarded_set(deps.storage, new_rewarded_set, expected_active_set_size)?;

    Ok(response.add_event(new_advance_epoch_event(updated_interval, num_nodes as u32)))
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

    let interval = storage::current_interval(deps.storage)?;
    if force_immediately || interval.is_current_interval_over(&env) {
        change_interval_config(
            deps.storage,
            interval,
            epochs_in_interval,
            epoch_duration_secs,
        )
    } else {
        // push the interval event
        let interval_event = PendingIntervalEventData::UpdateIntervalConfig {
            epochs_in_interval,
            epoch_duration_secs,
        };
        push_new_interval_event(deps.storage, &interval_event)?;
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
    use mixnet_contract_common::pending_events::PendingEpochEventData;
    use vesting_contract_common::messages::ExecuteMsg as VestingContractExecuteMsg;

    fn push_n_dummy_epoch_actions(test: &mut TestSetup, n: usize) {
        // if you attempt to undelegate non-existent delegation,
        // it will return an empty response, but will not fail
        for i in 0..n {
            let dummy_action = PendingEpochEventData::Undelegate {
                owner: Addr::unchecked("foomp"),
                mix_id: i as NodeId,
                proxy: None,
            };
            storage::push_new_epoch_event(test.deps_mut().storage, &dummy_action).unwrap();
        }
    }

    fn push_n_dummy_interval_actions(test: &mut TestSetup, n: usize) {
        // if you attempt to update cost parameters of an unbonded mixnode,
        // it will return an empty response, but will not fail
        for i in 0..n {
            let dummy_action = PendingIntervalEventData::ChangeMixCostParams {
                mix_id: i as NodeId,
                new_costs: fixtures::mix_node_cost_params_fixture(),
            };
            storage::push_new_interval_event(test.deps_mut().storage, &dummy_action).unwrap();
        }
    }

    #[cfg(test)]
    mod performing_pending_epoch_actions {
        use super::*;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use cosmwasm_std::{coin, coins, wasm_execute, Addr, BankMsg, Empty, SubMsg};
        use mixnet_contract_common::events::{
            new_active_set_update_event, new_delegation_on_unbonded_node_event,
            new_undelegation_event,
        };
        use mixnet_contract_common::pending_events::PendingEpochEventData;

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
            let action_with_event = PendingEpochEventData::UpdateActiveSetSize { new_size: 50 };
            storage::push_new_epoch_event(test.deps_mut().storage, &action_with_event).unwrap();
            push_n_dummy_epoch_actions(&mut test, 10);
            let (res, executed) =
                perform_pending_epoch_actions(test.deps_mut(), &env, None).unwrap();
            assert_eq!(
                res,
                Response::new().add_event(new_active_set_update_event(50))
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
            let vesting_contract = test.vesting_contract();

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
            let non_existent_delegation = PendingEpochEventData::Delegate {
                owner: Addr::unchecked("foomp"),
                mix_id: 123,
                amount: coin(123, TEST_COIN_DENOM),
                proxy: None,
            };
            storage::push_new_epoch_event(test.deps_mut().storage, &non_existent_delegation)
                .unwrap();
            expected_events.push(new_delegation_on_unbonded_node_event(
                &Addr::unchecked("foomp"),
                &None,
                123,
            ));
            expected_messages.push(SubMsg::new(BankMsg::Send {
                to_address: "foomp".to_string(),
                amount: coins(123, TEST_COIN_DENOM),
            }));

            // delegation to node that doesn't exist with vesting contract
            // we expect the same as above PLUS TrackUndelegation message
            let non_existent_delegation = PendingEpochEventData::Delegate {
                owner: Addr::unchecked("foomp2"),
                mix_id: 123,
                amount: coin(123, TEST_COIN_DENOM),
                proxy: Some(vesting_contract.clone()),
            };
            storage::push_new_epoch_event(test.deps_mut().storage, &non_existent_delegation)
                .unwrap();
            expected_events.push(new_delegation_on_unbonded_node_event(
                &Addr::unchecked("foomp2"),
                &Some(vesting_contract.clone()),
                123,
            ));
            expected_messages.push(SubMsg::new(BankMsg::Send {
                to_address: vesting_contract.clone().into_string(),
                amount: coins(123, TEST_COIN_DENOM),
            }));
            let msg = VestingContractExecuteMsg::TrackUndelegation {
                owner: "foomp2".to_string(),
                mix_id: 123,
                amount: coin(123, TEST_COIN_DENOM),
            };
            let track_undelegate_message = wasm_execute(vesting_contract, &msg, vec![]).unwrap();
            expected_messages.push(SubMsg::new(track_undelegate_message));

            // updating active set should only emit events and no cosmos messages
            let action_with_event = PendingEpochEventData::UpdateActiveSetSize { new_size: 50 };
            storage::push_new_epoch_event(test.deps_mut().storage, &action_with_event).unwrap();
            expected_events.push(new_active_set_update_event(50));

            // undelegation just returns tokens and emits event
            let legit_undelegate = PendingEpochEventData::Undelegate {
                owner: delegator.clone(),
                mix_id: legit_mix,
                proxy: None,
            };
            storage::push_new_epoch_event(test.deps_mut().storage, &legit_undelegate).unwrap();
            expected_events.push(new_undelegation_event(&delegator, &None, legit_mix));
            expected_messages.push(SubMsg::new(BankMsg::Send {
                to_address: delegator.into_string(),
                amount: coins(amount, TEST_COIN_DENOM),
            }));

            let (res, executed) =
                perform_pending_epoch_actions(test.deps_mut(), &env, None).unwrap();
            let mut expected = Response::new().add_events(expected_events);
            expected.messages = expected_messages;
            assert_eq!(res, expected);
            assert_eq!(executed, 4);
            assert_eq!(
                4,
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
            let action_with_event = PendingIntervalEventData::UpdateRewardingParams { update };
            storage::push_new_interval_event(test.deps_mut().storage, &action_with_event).unwrap();
            push_n_dummy_interval_actions(&mut test, 10);
            let (res, executed) =
                perform_pending_interval_actions(test.deps_mut(), &env, None).unwrap();
            let updated_params = test.rewarding_params().interval;
            assert_eq!(
                res,
                Response::new()
                    .add_event(new_rewarding_params_update_event(update, updated_params))
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
            let cost_change = PendingIntervalEventData::ChangeMixCostParams {
                mix_id: legit_mix,
                new_costs: new_costs.clone(),
            };

            storage::push_new_interval_event(test.deps_mut().storage, &cost_change).unwrap();
            expected_events.push(new_mixnode_cost_params_update_event(legit_mix, &new_costs));

            let update = IntervalRewardingParamsUpdate {
                rewarded_set_size: Some(500),
                ..Default::default()
            };
            let change_params = PendingIntervalEventData::UpdateRewardingParams { update };
            storage::push_new_interval_event(test.deps_mut().storage, &change_params).unwrap();
            let interval = test.current_interval();
            let mut expected_updated = test.rewarding_params();
            expected_updated
                .try_apply_updates(update, interval.epochs_in_interval())
                .unwrap();
            expected_events.push(new_rewarding_params_update_event(
                update,
                expected_updated.interval,
            ));

            let change_interval = PendingIntervalEventData::UpdateIntervalConfig {
                epochs_in_interval: 123,
                epoch_duration_secs: 1000,
            };
            let mut expected_updated2 = expected_updated;
            expected_updated2.apply_epochs_in_interval_change(123);
            storage::push_new_interval_event(test.deps_mut().storage, &change_interval).unwrap();
            expected_events.push(new_interval_config_update_event(
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
    mod reconciling_epoch_events {
        use super::*;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use cosmwasm_std::{coin, coins, BankMsg, Empty, SubMsg};
        use mixnet_contract_common::events::{
            new_delegation_on_unbonded_node_event, new_rewarding_params_update_event,
        };
        use mixnet_contract_common::pending_events::PendingEpochEventData;
        use mixnet_contract_common::reward_params::IntervalRewardingParamsUpdate;

        #[test]
        fn returns_error_if_epoch_is_in_progress() {
            let mut test = TestSetup::new();
            let env = test.env();

            let res = try_reconcile_epoch_events(test.deps_mut(), env, None);
            assert!(matches!(
                res,
                Err(MixnetContractError::EpochInProgress { .. })
            ))
        }

        #[test]
        fn only_clears_epoch_events_if_interval_is_in_progress() {
            let mut test = TestSetup::new();

            push_n_dummy_epoch_actions(&mut test, 10);
            push_n_dummy_interval_actions(&mut test, 10);
            test.skip_to_current_epoch_end();

            let env = test.env();
            try_reconcile_epoch_events(test.deps_mut(), env, None).unwrap();

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

            let env = test.env();
            try_reconcile_epoch_events(test.deps_mut(), env, None).unwrap();

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

            try_reconcile_epoch_events(test1.deps_mut(), env.clone(), Some(5)).unwrap();
            let epoch_events = test1.pending_epoch_events();
            let interval_events = test1.pending_interval_events();
            assert_eq!(epoch_events.len(), 5);
            assert_eq!(interval_events.len(), 10);

            try_reconcile_epoch_events(test1.deps_mut(), env.clone(), Some(7)).unwrap();
            let epoch_events = test1.pending_epoch_events();
            let interval_events = test1.pending_interval_events();
            assert!(epoch_events.is_empty());
            assert_eq!(interval_events.len(), 8);

            try_reconcile_epoch_events(test1.deps_mut(), env.clone(), Some(7)).unwrap();
            let epoch_events = test1.pending_epoch_events();
            let interval_events = test1.pending_interval_events();
            assert!(epoch_events.is_empty());
            assert_eq!(interval_events.len(), 1);

            try_reconcile_epoch_events(test1.deps_mut(), env.clone(), Some(7)).unwrap();
            let epoch_events = test1.pending_epoch_events();
            let interval_events = test1.pending_interval_events();
            assert!(epoch_events.is_empty());
            assert!(interval_events.is_empty());

            try_reconcile_epoch_events(test2.deps_mut(), env.clone(), Some(15)).unwrap();
            let epoch_events = test2.pending_epoch_events();
            let interval_events = test2.pending_interval_events();
            assert!(epoch_events.is_empty());
            assert_eq!(interval_events.len(), 5);

            try_reconcile_epoch_events(test2.deps_mut(), env.clone(), Some(15)).unwrap();
            let epoch_events = test2.pending_epoch_events();
            let interval_events = test2.pending_interval_events();
            assert!(epoch_events.is_empty());
            assert!(interval_events.is_empty());

            try_reconcile_epoch_events(test3.deps_mut(), env.clone(), Some(20)).unwrap();
            let epoch_events = test3.pending_epoch_events();
            let interval_events = test3.pending_interval_events();
            assert!(epoch_events.is_empty());
            assert!(interval_events.is_empty());

            try_reconcile_epoch_events(test4.deps_mut(), env, Some(100)).unwrap();
            let epoch_events = test4.pending_epoch_events();
            let interval_events = test4.pending_interval_events();
            assert!(epoch_events.is_empty());
            assert!(interval_events.is_empty());
        }

        #[test]
        fn catches_all_emitted_cosmos_events_and_messages() {
            let mut test = TestSetup::new();

            let mut expected_events = vec![new_reconcile_pending_events()];
            let mut expected_messages: Vec<SubMsg<Empty>> = Vec::new();

            // epoch event
            let non_existent_delegation = PendingEpochEventData::Delegate {
                owner: Addr::unchecked("foomp"),
                mix_id: 123,
                amount: coin(123, TEST_COIN_DENOM),
                proxy: None,
            };
            storage::push_new_epoch_event(test.deps_mut().storage, &non_existent_delegation)
                .unwrap();
            expected_events.push(new_delegation_on_unbonded_node_event(
                &Addr::unchecked("foomp"),
                &None,
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
            let change_params = PendingIntervalEventData::UpdateRewardingParams { update };
            storage::push_new_interval_event(test.deps_mut().storage, &change_params).unwrap();
            let interval = test.current_interval();
            let mut expected_updated = test.rewarding_params();
            expected_updated
                .try_apply_updates(update, interval.epochs_in_interval())
                .unwrap();
            expected_events.push(new_rewarding_params_update_event(
                update,
                expected_updated.interval,
            ));
            expected_events.push(new_pending_interval_events_execution_event(1));

            test.skip_to_current_interval_end();
            let env = test.env();
            let res = try_reconcile_epoch_events(test.deps_mut(), env, None).unwrap();
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
            MixnetContractError::DuplicateRewardedSetNode { node_id: 1 }
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
            MixnetContractError::DuplicateRewardedSetNode { node_id: 5 }
        );
    }

    #[cfg(test)]
    mod advancing_epoch {
        use super::*;
        use crate::rewards::models::RewardPoolChange;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use cosmwasm_std::testing::mock_info;
        use cosmwasm_std::{coin, coins, BankMsg, Decimal, Empty, SubMsg};
        use mixnet_contract_common::events::{
            new_delegation_on_unbonded_node_event, new_rewarding_params_update_event,
        };
        use mixnet_contract_common::reward_params::IntervalRewardingParamsUpdate;
        use mixnet_contract_common::RewardedSetNodeStatus;

        #[test]
        fn can_only_be_performed_by_specified_rewarding_validator() {
            let mut test = TestSetup::new();
            let current_active_set = test.rewarding_params().active_set_size;
            let some_sender = mock_info("foomper", &[]);

            test.skip_to_current_epoch_end();

            let env = test.env();
            let res = try_advance_epoch(
                test.deps_mut(),
                env,
                some_sender,
                vec![1, 2, 3],
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
                vec![1, 2, 3],
                current_active_set,
            );
            assert!(res.is_ok())
        }

        #[test]
        fn can_only_be_performed_if_epoch_is_over() {
            let mut test = TestSetup::new();
            let current_active_set = test.rewarding_params().active_set_size;

            let env = test.env();
            let sender = test.rewarding_validator();
            let res = try_advance_epoch(
                test.deps_mut(),
                env,
                sender.clone(),
                vec![1, 2, 3],
                current_active_set,
            );
            assert!(matches!(
                res,
                Err(MixnetContractError::EpochInProgress { .. })
            ));

            // sanity check
            test.skip_to_current_epoch_end();
            let env = test.env();
            let res = try_advance_epoch(
                test.deps_mut(),
                env,
                sender,
                vec![1, 2, 3],
                current_active_set,
            );
            assert!(res.is_ok())
        }

        #[test]
        fn only_clears_epoch_events_if_interval_is_in_progress() {
            let mut test = TestSetup::new();
            let current_active_set = test.rewarding_params().active_set_size;

            push_n_dummy_epoch_actions(&mut test, 10);
            push_n_dummy_interval_actions(&mut test, 10);
            test.skip_to_current_epoch_end();

            let env = test.env();
            let sender = test.rewarding_validator();
            try_advance_epoch(
                test.deps_mut(),
                env,
                sender,
                vec![1, 2, 3],
                current_active_set,
            )
            .unwrap();

            let epoch_events = test.pending_epoch_events();
            let interval_events = test.pending_interval_events();
            assert!(epoch_events.is_empty());
            assert_eq!(interval_events.len(), 10);
        }

        #[test]
        fn clears_both_epoch_and_interval_events_if_interval_has_finished() {
            let mut test = TestSetup::new();
            let current_active_set = test.rewarding_params().active_set_size;

            push_n_dummy_epoch_actions(&mut test, 10);
            push_n_dummy_interval_actions(&mut test, 10);
            test.skip_to_current_interval_end();

            let env = test.env();
            let sender = test.rewarding_validator();
            try_advance_epoch(
                test.deps_mut(),
                env,
                sender,
                vec![1, 2, 3],
                current_active_set,
            )
            .unwrap();

            let epoch_events = test.pending_epoch_events();
            let interval_events = test.pending_interval_events();
            assert!(epoch_events.is_empty());
            assert!(interval_events.is_empty());
        }

        #[test]
        fn if_executes_any_events_it_propagates_responses() {
            let mut test = TestSetup::new();
            let current_active_set = test.rewarding_params().active_set_size;

            let mut expected_events = Vec::new();
            let mut expected_messages: Vec<SubMsg<Empty>> = Vec::new();

            let non_existent_delegation = PendingEpochEventData::Delegate {
                owner: Addr::unchecked("foomp"),
                mix_id: 123,
                amount: coin(123, TEST_COIN_DENOM),
                proxy: None,
            };
            storage::push_new_epoch_event(test.deps_mut().storage, &non_existent_delegation)
                .unwrap();
            expected_events.push(new_delegation_on_unbonded_node_event(
                &Addr::unchecked("foomp"),
                &None,
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
            let change_params = PendingIntervalEventData::UpdateRewardingParams { update };
            storage::push_new_interval_event(test.deps_mut().storage, &change_params).unwrap();
            let interval = test.current_interval();
            let mut expected_updated = test.rewarding_params();
            expected_updated
                .try_apply_updates(update, interval.epochs_in_interval())
                .unwrap();
            expected_events.push(new_rewarding_params_update_event(
                update,
                expected_updated.interval,
            ));
            expected_events.push(new_pending_interval_events_execution_event(1));
            let current_interval = test.current_interval();
            let expected = current_interval.advance_epoch();
            expected_events.push(new_advance_epoch_event(expected, 3));

            test.skip_to_current_interval_end();

            let env = test.env();
            let sender = test.rewarding_validator();
            let res = try_advance_epoch(
                test.deps_mut(),
                env,
                sender,
                vec![1, 2, 3],
                current_active_set,
            )
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

        #[test]
        fn if_interval_is_over_applies_reward_pool_changes() {
            let mut test = TestSetup::new();
            let current_active_set = test.rewarding_params().active_set_size;

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

            // end of epoch - nothing has happened
            let sender = test.rewarding_validator();
            test.skip_to_current_epoch_end();
            let env = test.env();
            try_advance_epoch(
                test.deps_mut(),
                env,
                sender,
                vec![1, 2, 3],
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
            let env = test.env();
            try_advance_epoch(
                test.deps_mut(),
                env,
                sender,
                vec![1, 2, 3],
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
            let current_active_set = test.rewarding_params().active_set_size;

            let interval_pre = test.current_interval();
            let rewarded_set_pre = test.rewarded_set();
            assert!(rewarded_set_pre.is_empty());

            let sender = test.rewarding_validator();
            test.skip_to_current_interval_end();
            let env = test.env();
            try_advance_epoch(
                test.deps_mut(),
                env,
                sender,
                vec![1, 2, 3],
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
            assert!(matches!(events[0],
                PendingIntervalEventData::UpdateIntervalConfig { epochs_in_interval, epoch_duration_secs } if epochs_in_interval == 100 && epoch_duration_secs == 1000
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

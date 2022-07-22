// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::interval::helpers::change_epochs_in_interval;
use crate::interval::pending_events::ContractExecutableEvent;
use crate::interval::storage::push_new_interval_event;
use crate::rewards;
use crate::rewards::storage as rewards_storage;
use crate::support::helpers::{ensure_is_authorized, ensure_is_owner};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, Storage};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_advance_epoch_event, new_interval_config_update_event,
    new_pending_epoch_events_execution_event, new_pending_interval_config_update_event,
    new_pending_interval_events_execution_event, new_reconcile_pending_events,
};
use mixnet_contract_common::pending_events::PendingIntervalEvent;
use mixnet_contract_common::NodeId;
use std::time::Duration;

// those two should be called in separate tx (from advancing epoch),
// since there might be a lot of events to execute.
// however, it should also be called when advancing epoch itself in case somebody
// manage to sneak in a transaction between those two operations
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
    if interval.is_current_epoch_over(&env) {
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

// We've distributed the rewards to the rewarded set from the validator api before making this call (implicit order, should be solved in the future)
// We now write the new rewarded set, snapshot the mixnodes and finally reconcile all delegations and undelegations. That way the rewards for the previous
// epoch will be calculated correctly as the delegations and undelegations from the previous epoch will only take effect in the next (current) one.
fn update_rewarded_set(
    storage: &mut dyn Storage,
    new_rewarded_set: Vec<NodeId>,
    active_set_size: u32,
) -> Result<(), MixnetContractError> {
    let reward_params = rewards_storage::REWARDING_PARAMS.load(storage)?;

    // We don't want more then we need, less should be fine, as we could have less nodes bonded overall
    if active_set_size > reward_params.active_set_size {
        return Err(MixnetContractError::UnexpectedActiveSetSize {
            received: active_set_size,
            expected: reward_params.active_set_size,
        });
    }

    if new_rewarded_set.len() as u32 > reward_params.rewarded_set_size {
        return Err(MixnetContractError::UnexpectedRewardedSetSize {
            received: new_rewarded_set.len() as u32,
            expected: reward_params.rewarded_set_size,
        });
    }

    Ok(storage::update_rewarded_set(
        storage,
        active_set_size,
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
        let (mut sub_response, _) = perform_pending_epoch_actions(deps.branch(), &env, None)?;
        response.messages.append(&mut sub_response.messages);
        response.attributes.append(&mut sub_response.attributes);
        response.events.append(&mut sub_response.events);
    }

    // first clear epoch events queue and then touch the interval actions
    if current_interval.is_current_interval_over(&env) {
        // the interval has finished -> we can change things such as the profit margin
        let (mut sub_response, _) = perform_pending_interval_actions(deps.branch(), &env, None)?;
        response.messages.append(&mut sub_response.messages);
        response.attributes.append(&mut sub_response.attributes);
        response.events.append(&mut sub_response.events);

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

    let mut interval = storage::current_interval(deps.storage)?;
    if force_immediately || interval.is_current_interval_over(&env) {
        interval.change_epoch_length(Duration::from_secs(epoch_duration_secs));
        change_epochs_in_interval(deps.storage, interval, epochs_in_interval)?;
        Ok(Response::new().add_event(new_interval_config_update_event(
            epochs_in_interval,
            epoch_duration_secs,
        )))
    } else {
        // push the interval event
        let interval_event = PendingIntervalEvent::UpdateIntervalConfig {
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
}

//
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::support::tests::test_helpers;
//     use cosmwasm_std::testing::{mock_env, mock_info};
//     use cosmwasm_std::Timestamp;
//     use mixnet_contract_common::RewardedSetNodeStatus;
//     use mixnet_params_storage::rewarding_validator_address;
//
//     #[test]
//     fn writing_rewarded_set() {
//         let mut env = mock_env();
//         let mut deps = test_helpers::init_contract();
//         let current_state = mixnet_params_storage::CONTRACT_STATE
//             .load(deps.as_mut().storage)
//             .unwrap();
//         let authorised_sender = mock_info(current_state.rewarding_validator_address.as_str(), &[]);
//         let full_rewarded_set = (0..current_state.params.mixnode_rewarded_set_size)
//             .map(|i| format!("identity{:04}", i))
//             .collect::<Vec<_>>();
//         let last_update = 123;
//         storage::CURRENT_REWARDED_SET_HEIGHT
//             .save(deps.as_mut().storage, &last_update)
//             .unwrap();
//
//         // can only be performed by the permitted validator
//         let dummy_sender = mock_info("dummy_sender", &[]);
//         assert_eq!(
//             Err(ContractError::Unauthorized),
//             try_write_rewarded_set(
//                 deps.as_mut(),
//                 env.clone(),
//                 dummy_sender,
//                 full_rewarded_set.clone(),
//                 current_state.params.mixnode_active_set_size
//             )
//         );
//
//         // the sender must use the same active set size as the one defined in the contract
//         assert_eq!(
//             Err(ContractError::UnexpectedActiveSetSize {
//                 received: 123,
//                 expected: current_state.params.mixnode_active_set_size
//             }),
//             try_write_rewarded_set(
//                 deps.as_mut(),
//                 env.clone(),
//                 authorised_sender.clone(),
//                 full_rewarded_set.clone(),
//                 123
//             )
//         );
//
//         // the sender cannot provide more nodes than the rewarded set size
//         let mut bigger_set = full_rewarded_set.clone();
//         bigger_set.push("another_node".to_string());
//         assert_eq!(
//             Err(ContractError::UnexpectedRewardedSetSize {
//                 received: current_state.params.mixnode_rewarded_set_size + 1,
//                 expected: current_state.params.mixnode_rewarded_set_size
//             }),
//             try_write_rewarded_set(
//                 deps.as_mut(),
//                 env.clone(),
//                 authorised_sender.clone(),
//                 bigger_set,
//                 current_state.params.mixnode_active_set_size
//             )
//         );
//
//         // cannot be performed too soon after a previous update
//         env.block.height = last_update + 1;
//         // after successful rewarded set write, all internal storage structures are updated appropriately
//         env.block.height = last_update + crate::constants::REWARDED_SET_REFRESH_BLOCKS;
//         let expected_response = Response::new().add_event(new_change_rewarded_set_event(
//             current_state.params.mixnode_active_set_size,
//             current_state.params.mixnode_rewarded_set_size,
//             full_rewarded_set.len() as u32,
//         ));
//
//         assert_eq!(
//             Ok(expected_response),
//             try_write_rewarded_set(
//                 deps.as_mut(),
//                 env.clone(),
//                 authorised_sender,
//                 full_rewarded_set.clone(),
//                 current_state.params.mixnode_active_set_size
//             )
//         );
//
//         for (i, rewarded_node) in full_rewarded_set.into_iter().enumerate() {
//             if (i as u32) < current_state.params.mixnode_active_set_size {
//                 assert_eq!(
//                     RewardedSetNodeStatus::Active,
//                     storage::REWARDED_SET
//                         .load(deps.as_ref().storage, (env.block.height, rewarded_node))
//                         .unwrap()
//                 )
//             } else {
//                 assert_eq!(
//                     RewardedSetNodeStatus::Standby,
//                     storage::REWARDED_SET
//                         .load(deps.as_ref().storage, (env.block.height, rewarded_node))
//                         .unwrap()
//                 )
//             }
//         }
//         assert_eq!(
//             env.block.height,
//             storage::CURRENT_REWARDED_SET_HEIGHT
//                 .load(deps.as_ref().storage)
//                 .unwrap()
//         );
//     }
//
//     #[test]
//     fn advancing_epoch() {
//         let mut env = mock_env();
//         let mut deps = test_helpers::init_contract();
//         let sender = rewarding_validator_address(&deps.storage).unwrap();
//
//         let _current_epoch = init_epoch(&mut deps.storage, env.clone()).unwrap();
//
//         // Works as its after the current epoch
//         env.block.time = Timestamp::from_seconds(1641081600);
//         assert!(try_advance_epoch(env.clone(), deps.as_mut().storage, sender.clone()).is_ok());
//
//         let current_epoch = crate::interval::storage::current_epoch(&mut deps.storage).unwrap();
//
//         // same if the current blocktime is set to BEFORE the first interval has even begun
//         // (say we decided to set the first interval to be some time in the future at initialisation)
//         env.block.time = Timestamp::from_seconds(1609459200);
//         assert_eq!(
//             Err(ContractError::EpochInProgress {
//                 current_block_time: 1609459200,
//                 epoch_start: current_epoch.start_unix_timestamp(),
//                 epoch_end: current_epoch.end_unix_timestamp()
//             }),
//             try_advance_epoch(env.clone(), deps.as_mut().storage, sender.clone(),)
//         );
//
//         // works otherwise
//
//         // interval that has just finished
//         env.block.time =
//             Timestamp::from_seconds(current_epoch.start_unix_timestamp() as u64 + 10000);
//         let expected_new_epoch = current_epoch.next_on_chain(env.clone());
//         let expected_response =
//             Response::new().add_event(new_advance_interval_event(expected_new_epoch));
//         assert_eq!(
//             Ok(expected_response),
//             try_advance_epoch(env.clone(), deps.as_mut().storage, sender)
//         );
//
//         // interval way back in the past (i.e. 'somebody' failed to advance it for a long time)
//         env.block.time = Timestamp::from_seconds(1672531200);
//         storage::save_epoch(deps.as_mut().storage, &current_epoch).unwrap();
//         let expected_new_epoch = current_epoch.next_on_chain(env.clone());
//         let expected_response =
//             Response::new().add_event(new_advance_interval_event(expected_new_epoch));
//         let sender = rewarding_validator_address(&deps.storage).unwrap();
//         assert_eq!(
//             Ok(expected_response),
//             try_advance_epoch(env.clone(), deps.as_mut().storage, sender)
//         );
//     }
// }

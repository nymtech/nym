// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::delegations::storage as delegations_storage;
use crate::interval::storage as interval_storage;
use crate::interval::storage::{push_new_epoch_event, push_new_interval_event};
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::helpers::get_mixnode_details_by_owner;
use crate::rewards::helpers;
use crate::support::helpers::{
    ensure_bonded, ensure_is_authorized, ensure_proxy_match, send_to_proxy_or_owner,
};
use cosmwasm_std::{wasm_execute, Addr, Coin, DepsMut, Env, MessageInfo, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_mix_rewarding_event, new_not_found_mix_operator_rewarding_event,
    new_zero_uptime_mix_operator_rewarding_event,
};
use mixnet_contract_common::pending_events::{PendingEpochEvent, PendingIntervalEvent};
use mixnet_contract_common::reward_params::{
    IntervalRewardingParamsUpdate, NodeRewardParams, Performance,
};
use mixnet_contract_common::{Delegation, NodeId};
use vesting_contract_common::messages::ExecuteMsg as VestingContractExecuteMsg;

pub(crate) fn try_reward_mixnode(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    node_id: NodeId,
    node_performance: Performance,
) -> Result<Response, MixnetContractError> {
    ensure_is_authorized(info.sender, deps.storage)?;

    // see if the epoch has finished
    let interval = interval_storage::current_interval(deps.storage)?;
    if !interval.is_current_epoch_over(&env) {
        return Err(MixnetContractError::EpochInProgress {
            current_block_time: env.block.time.seconds(),
            epoch_start: interval.current_epoch_start_unix_timestamp(),
            epoch_end: interval.current_epoch_end_unix_timestamp(),
        });
    }

    // there's a chance of this failing to load the details if the mixnode unbonded before rewards
    // were distributed and all of its delegators are also gone
    let mut mix_rewarding = match storage::MIXNODE_REWARDING.may_load(deps.storage, node_id)? {
        Some(mix_rewarding) if mix_rewarding.still_bonded() => mix_rewarding,
        // don't fail if the node has unbonded as we don't want to fail the underlying transaction
        _ => {
            return Ok(
                Response::new().add_event(new_not_found_mix_operator_rewarding_event(
                    interval, node_id,
                )),
            );
        }
    };

    // check if this node has already been rewarded for the current epoch.
    // unlike the previous check, this one should be a hard error since this cannot be
    // influenced by users actions
    let epoch_details = interval.current_full_epoch_id();
    if epoch_details == mix_rewarding.last_rewarded_epoch {
        return Err(MixnetContractError::MixnodeAlreadyRewarded {
            node_id,
            epoch_details,
        });
    }

    // again a hard error since the rewarding validator should have known not to reward this node
    let node_status = interval_storage::REWARDED_SET
        .load(deps.storage, node_id)
        .map_err(|_| MixnetContractError::MixnodeNotInRewardedSet {
            node_id,
            epoch_details,
        })?;

    // no need to calculate anything as rewards are going to be 0 for everything
    // however, we still need to update last_rewarded_epoch field
    if node_performance.is_zero() {
        mix_rewarding.last_rewarded_epoch = epoch_details;
        storage::MIXNODE_REWARDING.save(deps.storage, node_id, &mix_rewarding)?;
        return Ok(
            Response::new().add_event(new_zero_uptime_mix_operator_rewarding_event(
                interval, node_id,
            )),
        );
    }

    let rewarding_params = storage::REWARDING_PARAMS.load(deps.storage)?;
    let node_reward_params = NodeRewardParams::new(node_performance, node_status.is_active());

    // calculate each step separate for easier accounting
    let node_reward = mix_rewarding.node_reward(&rewarding_params, node_reward_params);
    let reward_distribution = mix_rewarding.determine_reward_split(
        node_reward,
        node_performance,
        interval.epochs_in_interval(),
    );
    mix_rewarding.distribute_rewards(reward_distribution, epoch_details);

    // persist changes happened to the storage
    storage::MIXNODE_REWARDING.save(deps.storage, node_id, &mix_rewarding)?;
    storage::reward_accounting(deps.storage, node_reward)?;

    Ok(Response::new().add_event(new_mix_rewarding_event(
        interval,
        node_id,
        reward_distribution,
    )))
}

pub(crate) fn try_withdraw_operator_reward(
    deps: DepsMut<'_>,
    info: MessageInfo,
) -> Result<Response, MixnetContractError> {
    _try_withdraw_operator_reward(deps, info.sender, None, info.funds)
}

pub(crate) fn try_withdraw_operator_reward_on_behalf(
    deps: DepsMut<'_>,
    info: MessageInfo,
    owner: String,
) -> Result<Response, MixnetContractError> {
    let proxy = info.sender;
    let owner = deps.api.addr_validate(&owner)?;
    _try_withdraw_operator_reward(deps, owner, Some(proxy), info.funds)
}

pub(crate) fn _try_withdraw_operator_reward(
    deps: DepsMut<'_>,
    owner: Addr,
    proxy: Option<Addr>,
    funds: Vec<Coin>,
) -> Result<Response, MixnetContractError> {
    // we need to grab all of the node's details so we'd known original pledge alongside
    // all the earned rewards (and obviously to know if this node even exists and is still
    // in the bonded state)
    let mix_details = get_mixnode_details_by_owner(deps.storage, owner.clone())?.ok_or(
        MixnetContractError::NoAssociatedMixNodeBond {
            owner: owner.clone(),
        },
    )?;

    ensure_proxy_match(&proxy, &mix_details.bond_information.proxy)?;
    ensure_bonded(&mix_details.bond_information)?;

    let reward = helpers::withdraw_operator_reward(deps.storage, mix_details)?;
    let return_tokens = send_to_proxy_or_owner(&proxy, &owner, vec![reward.clone()]);
    let mut response = Response::new().add_message(return_tokens);

    if let Some(proxy) = &proxy {
        // we can only attempt to send the message to the vesting contract if the proxy IS the vesting contract
        // otherwise, we don't care
        let vesting_contract = mixnet_params_storage::vesting_contract_address(deps.storage)?;
        if proxy == &vesting_contract {
            // TODO: ask @DU if this is the intended way of using "TrackReward" (are we supposed
            // to use the same call for both operator and delegators?
            let msg = VestingContractExecuteMsg::TrackReward {
                amount: reward,
                address: owner.into_string(),
            };
            let track_reward_message = wasm_execute(proxy, &msg, funds)?;
            response = response.add_message(track_reward_message);
        }
    }

    // TODO: insert events and all of that
    Ok(response)
}

pub(crate) fn try_withdraw_delegator_reward(
    deps: DepsMut<'_>,
    info: MessageInfo,
    mix_id: NodeId,
) -> Result<Response, MixnetContractError> {
    _try_withdraw_delegator_reward(deps, mix_id, info.sender, None, info.funds)
}

pub(crate) fn try_withdraw_delegator_reward_on_behalf(
    deps: DepsMut<'_>,
    info: MessageInfo,
    mix_id: NodeId,
    owner: String,
) -> Result<Response, MixnetContractError> {
    let proxy = info.sender;
    let owner = deps.api.addr_validate(&owner)?;
    _try_withdraw_delegator_reward(deps, mix_id, owner, Some(proxy), info.funds)
}

pub(crate) fn _try_withdraw_delegator_reward(
    deps: DepsMut<'_>,
    mix_id: NodeId,
    owner: Addr,
    proxy: Option<Addr>,
    funds: Vec<Coin>,
) -> Result<Response, MixnetContractError> {
    // see if the delegation even exists
    let storage_key = Delegation::generate_storage_key(mix_id, &owner, proxy.as_ref());
    let delegation = match delegations_storage::delegations().may_load(deps.storage, storage_key)? {
        None => return Ok(Response::default()),
        Some(delegation) => delegation,
    };
    // grab associated mixnode rewarding details
    let mix_rewarding =
        storage::MIXNODE_REWARDING.may_load(deps.storage, mix_id)?.ok_or(MixnetContractError::InconsistentState {
            comment: "mixnode rewarding got removed from the storage whilst there's still an existing delegation"
                .into(),
        })?;

    ensure_proxy_match(&proxy, &delegation.proxy)?;

    let reward = helpers::withdraw_delegator_reward(deps.storage, delegation, mix_rewarding)?;
    let return_tokens = send_to_proxy_or_owner(&proxy, &owner, vec![reward.clone()]);
    let mut response = Response::new().add_message(return_tokens);

    if let Some(proxy) = &proxy {
        // we can only attempt to send the message to the vesting contract if the proxy IS the vesting contract
        // otherwise, we don't care
        let vesting_contract = mixnet_params_storage::vesting_contract_address(deps.storage)?;
        if proxy == &vesting_contract {
            // TODO: ask @DU if this is the intended way of using "TrackReward" (are we supposed
            // to use the same call for both operator and delegators?
            let msg = VestingContractExecuteMsg::TrackReward {
                amount: reward,
                address: owner.into_string(),
            };
            let track_reward_message = wasm_execute(proxy, &msg, funds)?;
            response = response.add_message(track_reward_message);
        }
    }

    // TODO: insert events and all of that
    Ok(response)
}

pub(crate) fn try_update_active_set_size(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    active_set_size: u32,
    force_immediately: bool,
) -> Result<Response, MixnetContractError> {
    ensure_is_authorized(info.sender, deps.storage)?;

    let mut rewarding_params = storage::REWARDING_PARAMS.load(deps.storage)?;
    if active_set_size == 0 {
        return Err(MixnetContractError::ZeroActiveSet);
    }

    if active_set_size > rewarding_params.rewarded_set_size {
        return Err(MixnetContractError::InvalidActiveSetSize);
    }

    let interval = interval_storage::current_interval(deps.storage)?;
    if force_immediately || interval.is_current_epoch_over(&env) {
        rewarding_params.try_change_active_set_size(active_set_size)?;
        storage::REWARDING_PARAMS.save(deps.storage, &rewarding_params)?;
    } else {
        // push the epoch event
        let epoch_event = PendingEpochEvent::UpdateActiveSetSize {
            new_size: active_set_size,
        };
        push_new_epoch_event(deps.storage, &epoch_event)?;
    }

    // TODO: slap events
    Ok(Response::new())
}

pub(crate) fn try_update_rewarding_params(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    updated_params: IntervalRewardingParamsUpdate,
    force_immediately: bool,
) -> Result<Response, MixnetContractError> {
    ensure_is_authorized(info.sender, deps.storage)?;

    if !updated_params.contains_updates() {
        return Err(MixnetContractError::EmptyParamsChangeMsg);
    }

    let interval = interval_storage::current_interval(deps.storage)?;
    if force_immediately || interval.is_current_interval_over(&env) {
        let mut rewarding_params = storage::REWARDING_PARAMS.load(deps.storage)?;
        rewarding_params.try_apply_updates(updated_params, interval.epochs_in_interval())?;
        storage::REWARDING_PARAMS.save(deps.storage, &rewarding_params)?;
    } else {
        // push the interval event
        let interval_event = PendingIntervalEvent::UpdateRewardingParams {
            update: updated_params,
        };
        push_new_interval_event(deps.storage, &interval_event)?;
    }

    // TODO: slap events
    Ok(Response::new())
}

//
// #[cfg(test)]
// pub mod tests {
//     use super::*;
//     use crate::delegations::transactions::{
//         _try_remove_delegation_from_mixnode, try_delegate_to_mixnode,
//     };
//     use crate::error::ContractError;
//     use crate::interval::storage::{
//         current_epoch_reward_params, save_epoch, save_epoch_reward_params,
//     };
//     use crate::interval::transactions::try_advance_epoch;
//     use crate::mixnet_contract_settings::storage::{
//         self as mixnet_params_storage, rewarding_validator_address,
//     };
//     use crate::mixnodes::storage as mixnodes_storage;
//     use crate::mixnodes::storage::StoredMixnodeBond;
//     use crate::rewards::transactions::try_reward_mixnode;
//     use crate::support::helpers::{current_operator_epoch_cost, epochs_in_interval};
//     use crate::support::tests;
//     use crate::support::tests::test_helpers;
//     use az::CheckedCast;
//     use config::defaults::MIX_DENOM;
//     use cosmwasm_std::testing::{mock_env, mock_info};
//     use cosmwasm_std::{coin, coins, Addr, StdError, Timestamp, Uint128};
//     use mixnet_contract_common::events::{
//         must_find_attribute, BOND_TOO_FRESH_VALUE, NO_REWARD_REASON_KEY,
//         OPERATOR_REWARDING_EVENT_TYPE,
//     };
//     use mixnet_contract_common::reward_params::{
//         EpochRewardParams, NodeRewardParams, RewardParams,
//     };
//     use mixnet_contract_common::{Delegation, IdentityKey, Interval, Layer, MixNode};
//     use time::OffsetDateTime;
//
//     #[test]
//     fn rewarding_mixnodes_with_incorrect_interval_id() {
//         let mut deps = test_helpers::init_contract();
//         let mut env = mock_env();
//         let sender = rewarding_validator_address(&deps.storage).unwrap();
//         let info = mock_info(&sender, &coins(1000, MIX_DENOM.base));
//         crate::interval::transactions::init_epoch(&mut deps.storage, env.clone()).unwrap();
//
//         // bond the node
//         let node_owner: Addr = Addr::unchecked("node-owner");
//         let node_identity = test_helpers::add_mixnode(
//             node_owner.as_str(),
//             tests::fixtures::good_mixnode_pledge(),
//             deps.as_mut(),
//         );
//
//         // Reward once
//         let res = try_reward_mixnode(
//             deps.as_mut(),
//             env.clone(),
//             info.clone(),
//             node_identity.clone(),
//             tests::fixtures::node_reward_params_fixture(100),
//         );
//         assert!(res.is_ok());
//
//         let res = try_reward_mixnode(
//             deps.as_mut(),
//             env.clone(),
//             info.clone(),
//             node_identity.clone(),
//             tests::fixtures::node_reward_params_fixture(100),
//         );
//         // Fails since mixnode was already rewarded in this epoch
//         assert!(res.is_err());
//
//         let rewarding_validator_address = rewarding_validator_address(&deps.storage).unwrap();
//
//         // Advance epoch
//         let premature_advance = try_advance_epoch(
//             env.clone(),
//             &mut deps.storage,
//             rewarding_validator_address.clone(),
//         );
//         assert!(premature_advance.is_err());
//
//         env.block.time = Timestamp::from_seconds(env.block.time.seconds() + 3600);
//
//         let timely_advance =
//             try_advance_epoch(env.clone(), &mut deps.storage, rewarding_validator_address);
//         assert!(timely_advance.is_ok());
//
//         let res = try_reward_mixnode(
//             deps.as_mut(),
//             env.clone(),
//             info.clone(),
//             node_identity.clone(),
//             tests::fixtures::node_reward_params_fixture(100),
//         );
//         assert!(res.is_ok());
//
//         test_helpers::update_env_and_progress_interval(&mut env, deps.as_mut().storage);
//
//         let res = try_reward_mixnode(
//             deps.as_mut(),
//             env.clone(),
//             info.clone(),
//             node_identity,
//             tests::fixtures::node_reward_params_fixture(100),
//         );
//         assert!(res.is_ok());
//     }
//
//     #[test]
//     fn attempting_rewarding_mixnode_multiple_times_per_interval() {
//         let mut deps = test_helpers::init_contract();
//         let mut env = mock_env();
//         let current_state = mixnet_params_storage::CONTRACT_STATE
//             .load(deps.as_mut().storage)
//             .unwrap();
//         let rewarding_validator_address = current_state.rewarding_validator_address;
//
//         // bond the node
//         let node_owner: Addr = Addr::unchecked("node-owner");
//         let node_identity = test_helpers::add_mixnode(
//             node_owner.as_str(),
//             tests::fixtures::good_mixnode_pledge(),
//             deps.as_mut(),
//         );
//
//         let info = mock_info(rewarding_validator_address.as_ref(), &[]);
//
//         // first reward goes through just fine
//         let res = try_reward_mixnode(
//             deps.as_mut(),
//             env.clone(),
//             info.clone(),
//             node_identity.clone(),
//             tests::fixtures::node_reward_params_fixture(100),
//         );
//         assert!(res.is_ok());
//
//         // but the other one fails
//         let res = try_reward_mixnode(
//             deps.as_mut(),
//             env.clone(),
//             info.clone(),
//             node_identity.clone(),
//             tests::fixtures::node_reward_params_fixture(100),
//         );
//         assert_eq!(
//             Err(ContractError::MixnodeAlreadyRewarded {
//                 identity: node_identity.clone()
//             }),
//             res
//         );
//
//         // but rewarding the same node in the following interval is fine again
//         test_helpers::update_env_and_progress_interval(&mut env, deps.as_mut().storage);
//
//         let res = try_reward_mixnode(
//             deps.as_mut(),
//             env,
//             info.clone(),
//             node_identity,
//             tests::fixtures::node_reward_params_fixture(100),
//         );
//         assert!(res.is_ok());
//     }
//
//     #[test]
//     fn rewarding_mixnode_blockstamp_based() {
//         let mut deps = test_helpers::init_contract();
//
//         let mut env = mock_env();
//         let current_state = mixnet_params_storage::CONTRACT_STATE
//             .load(deps.as_mut().storage)
//             .unwrap();
//         let rewarding_validator_address = current_state.rewarding_validator_address;
//
//         let node_owner: Addr = Addr::unchecked("node-owner");
//         let node_identity: IdentityKey = "nodeidentity".into();
//
//         let initial_bond = 10000_000000;
//         let initial_delegation = 20000_000000;
//         let mixnode_bond = StoredMixnodeBond {
//             pledge_amount: coin(initial_bond, MIX_DENOM.base),
//             owner: node_owner,
//             layer: Layer::One,
//             block_height: env.block.height,
//             mix_node: MixNode {
//                 identity_key: node_identity.clone(),
//                 ..tests::fixtures::mix_node_fixture()
//             },
//             proxy: None,
//             accumulated_rewards: None,
//             epoch_rewards: None,
//         };
//
//         mixnodes_storage::mixnodes()
//             .save(
//                 deps.as_mut().storage,
//                 &node_identity,
//                 &mixnode_bond,
//                 env.block.height,
//             )
//             .unwrap();
//         mixnodes_storage::TOTAL_DELEGATION
//             .save(
//                 deps.as_mut().storage,
//                 &node_identity,
//                 &Uint128::new(initial_delegation),
//             )
//             .unwrap();
//
//         // delegation happens later, but not later enough
//         env.block.height += constants::MINIMUM_BLOCK_AGE_FOR_REWARDING - 1;
//         env.block.time = Timestamp::from_seconds(OffsetDateTime::now_utc().unix_timestamp() as u64);
//
//         delegations_storage::delegations()
//             .save(
//                 deps.as_mut().storage,
//                 (node_identity.clone(), "delegator".into(), env.block.height),
//                 &Delegation::new(
//                     Addr::unchecked("delegator"),
//                     node_identity.clone(),
//                     coin(initial_delegation, MIX_DENOM.base),
//                     env.block.height,
//                     None,
//                 ),
//             )
//             .unwrap();
//
//         let info = mock_info(rewarding_validator_address.as_ref(), &[]);
//
//         let epoch = Interval::init_epoch(env.clone());
//         save_epoch(&mut deps.storage, &epoch).unwrap();
//         save_epoch_reward_params(epoch.id(), &mut deps.storage).unwrap();
//
//         let epoch_from_storage = crate::interval::storage::current_epoch(&deps.storage).unwrap();
//         assert_eq!(epoch_from_storage.id(), 0);
//
//         let res = try_reward_mixnode(
//             deps.as_mut(),
//             env.clone(),
//             info.clone(),
//             node_identity.clone(),
//             tests::fixtures::node_reward_params_fixture(100),
//         )
//         .unwrap();
//
//         assert_eq!(
//             initial_bond,
//             test_helpers::read_mixnode_pledge_amount(deps.as_ref().storage, &node_identity)
//                 .unwrap()
//                 .u128()
//         );
//         assert_eq!(
//             initial_delegation,
//             mixnodes_storage::TOTAL_DELEGATION
//                 .load(deps.as_ref().storage, &node_identity)
//                 .unwrap()
//                 .u128()
//         );
//         assert_eq!(1, res.events.len());
//         assert_eq!(OPERATOR_REWARDING_EVENT_TYPE, res.events[0].ty);
//         assert_eq!(
//             BOND_TOO_FRESH_VALUE,
//             must_find_attribute(&res.events[0], NO_REWARD_REASON_KEY)
//         );
//
//         // reward can happen now, but only for bonded node
//         env.block.height += 1;
//         env.block.time = Timestamp::from_seconds(epoch.next().start_unix_timestamp() as u64);
//         let sender =
//             crate::mixnet_contract_settings::storage::rewarding_validator_address(&deps.storage)
//                 .unwrap();
//         try_advance_epoch(env.clone(), &mut deps.storage, sender).unwrap();
//
//         let info = mock_info(rewarding_validator_address.as_ref(), &[]);
//
//         let res = try_reward_mixnode(
//             deps.as_mut(),
//             env.clone(),
//             info.clone(),
//             node_identity.clone(),
//             tests::fixtures::node_reward_params_fixture(100),
//         )
//         .unwrap();
//
//         let mixnode = crate::mixnodes::storage::mixnodes()
//             .load(&deps.storage, &node_identity)
//             .unwrap();
//
//         assert!(mixnode.accumulated_rewards > Some(Uint128::zero()),);
//         assert_eq!(
//             initial_delegation,
//             mixnodes_storage::TOTAL_DELEGATION
//                 .load(deps.as_ref().storage, &node_identity)
//                 .unwrap()
//                 .u128()
//         );
//
//         assert_eq!(1, res.events.len());
//         let event = &res.events[0];
//         assert_eq!(OPERATOR_REWARDING_EVENT_TYPE, event.ty);
//         // assert_ne!("0", must_find_attribute(event, TOTAL_MIXNODE_REWARD_KEY));
//         // assert_ne!("0", must_find_attribute(event, OPERATOR_REWARD_KEY));
//         // assert_eq!(
//         //     "0",
//         //     must_find_attribute(event, DISTRIBUTED_DELEGATION_REWARDS_KEY)
//         // );
//         // assert_eq!(
//         //     false.to_string(),
//         //     must_find_attribute(event, FURTHER_DELEGATIONS_TO_REWARD_KEY)
//         // );
//
//         // reward happens now, both for node owner and delegators
//         env.block.height += constants::MINIMUM_BLOCK_AGE_FOR_REWARDING - 1;
//         test_helpers::update_env_and_progress_interval(&mut env, deps.as_mut().storage);
//
//         let pledge_before_rewarding =
//             test_helpers::read_mixnode_pledge_amount(deps.as_ref().storage, &node_identity)
//                 .unwrap()
//                 .u128();
//
//         let info = mock_info(rewarding_validator_address.as_ref(), &[]);
//         let res = try_reward_mixnode(
//             deps.as_mut(),
//             env,
//             info.clone(),
//             node_identity.clone(),
//             tests::fixtures::node_reward_params_fixture(100),
//         )
//         .unwrap();
//
//         // We are in a lazy system, rewarding will not increase pledge or delegations
//         assert_eq!(
//             test_helpers::read_mixnode_pledge_amount(deps.as_ref().storage, &node_identity)
//                 .unwrap()
//                 .u128(),
//             pledge_before_rewarding
//         );
//         assert_eq!(
//             mixnodes_storage::TOTAL_DELEGATION
//                 .load(deps.as_ref().storage, &node_identity)
//                 .unwrap()
//                 .u128(),
//             initial_delegation
//         );
//
//         assert_eq!(1, res.events.len());
//         let event = &res.events[0];
//         assert_eq!(OPERATOR_REWARDING_EVENT_TYPE, event.ty);
//         // assert_ne!("0", must_find_attribute(event, TOTAL_MIXNODE_REWARD_KEY));
//         // assert_ne!("0", must_find_attribute(event, OPERATOR_REWARD_KEY));
//         // assert_ne!(
//         //     "0",
//         //     must_find_attribute(event, DISTRIBUTED_DELEGATION_REWARDS_KEY)
//         // );
//         // assert_eq!(
//         //     false.to_string(),
//         //     must_find_attribute(event, FURTHER_DELEGATIONS_TO_REWARD_KEY)
//         // );
//     }
//
//     #[test]
//     fn test_reward_additivity_and_snapshots() {
//         use crate::constants::INTERVAL_REWARD_PERCENT;
//         use crate::contract::INITIAL_REWARD_POOL;
//         use crate::mixnodes::transactions::try_add_mixnode;
//         use rand::thread_rng;
//
//         let mixnodes = crate::mixnodes::storage::mixnodes();
//
//         type U128 = fixed::types::U75F53;
//
//         let mut deps = test_helpers::init_contract();
//         let mut env = mock_env();
//         let current_state = mixnet_params_storage::CONTRACT_STATE
//             .load(deps.as_ref().storage)
//             .unwrap();
//         let rewarding_validator_address = current_state.rewarding_validator_address;
//
//         let info = mock_info(rewarding_validator_address.as_str(), &[]);
//
//         crate::mixnodes::transactions::try_checkpoint_mixnodes(
//             &mut deps.storage,
//             env.block.height,
//             info.clone(),
//         )
//         .unwrap();
//         let checkpoints = mixnodes
//             .changelog()
//             .keys(&deps.storage, None, None, Order::Ascending)
//             .filter_map(|x| x.ok())
//             .collect::<Vec<(IdentityKey, u64)>>();
//         assert_eq!(0, checkpoints.len());
//
//         let period_reward_pool =
//             (INITIAL_REWARD_POOL / 100 / epochs_in_interval(&deps.storage).unwrap() as u128)
//                 * INTERVAL_REWARD_PERCENT as u128;
//         assert_eq!(period_reward_pool, 6_944_444_444);
//         let circulating_supply = storage::circulating_supply(&deps.storage).unwrap().u128();
//         assert_eq!(circulating_supply, 750_000_000_000_000u128);
//
//         let staking_supply = storage::staking_supply(&deps.storage).unwrap().u128();
//         assert_eq!(staking_supply, 100_000_000_000_000u128);
//
//         let sender = Addr::unchecked("alice");
//         let stake = coins(10_000_000_000, MIX_DENOM.base);
//
//         let keypair = crypto::asymmetric::identity::KeyPair::new(&mut thread_rng());
//         let owner_signature = keypair
//             .private_key()
//             .sign(sender.as_bytes())
//             .to_base58_string();
//
//         let legit_sphinx_key = crypto::asymmetric::encryption::KeyPair::new(&mut thread_rng());
//
//         let info = mock_info(sender.as_str(), &stake);
//
//         let node_identity_1 = keypair.public_key().to_base58_string();
//
//         try_add_mixnode(
//             deps.as_mut(),
//             env.clone(),
//             info.clone(),
//             MixNode {
//                 identity_key: node_identity_1.clone(),
//                 sphinx_key: legit_sphinx_key.public_key().to_base58_string(),
//                 ..tests::fixtures::mix_node_fixture()
//             },
//             owner_signature,
//         )
//         .unwrap();
//
//         // tick
//
//         env.block.height += 1;
//
//         let info = mock_info(rewarding_validator_address.as_str(), &[]);
//         crate::mixnodes::transactions::try_checkpoint_mixnodes(
//             &mut deps.storage,
//             env.block.height,
//             info.clone(),
//         )
//         .unwrap();
//         mixnodes
//             .assert_checkpointed(&deps.storage, env.block.height)
//             .unwrap();
//         let checkpoints = mixnodes
//             .changelog()
//             .keys(&deps.storage, None, None, Order::Ascending)
//             .filter_map(|x| x.ok())
//             .collect::<Vec<(IdentityKey, u64)>>();
//         assert_eq!(checkpoints.len(), 1);
//
//         let node_owner: Addr = Addr::unchecked("johnny");
//         let node_identity_2 = test_helpers::add_mixnode(
//             node_owner.as_str(),
//             coins(10_000_000_000, MIX_DENOM.base),
//             deps.as_mut(),
//         );
//
//         try_delegate_to_mixnode(
//             deps.as_mut(),
//             mock_env(),
//             mock_info("alice_d1", &[coin(8000_000000, MIX_DENOM.base)]),
//             node_identity_1.clone(),
//         )
//         .unwrap();
//
//         try_delegate_to_mixnode(
//             deps.as_mut(),
//             mock_env(),
//             mock_info("alice_d2", &[coin(2000_000000, MIX_DENOM.base)]),
//             node_identity_1.clone(),
//         )
//         .unwrap();
//
//         try_delegate_to_mixnode(
//             deps.as_mut(),
//             mock_env(),
//             mock_info("bob_d1", &[coin(8000_000000, MIX_DENOM.base)]),
//             node_identity_2.clone(),
//         )
//         .unwrap();
//
//         try_delegate_to_mixnode(
//             deps.as_mut(),
//             mock_env(),
//             mock_info("bob_d2", &[coin(2000_000000, MIX_DENOM.base)]),
//             node_identity_2.clone(),
//         )
//         .unwrap();
//
//         let node_owner: Addr = Addr::unchecked("alicebob");
//         let node_identity_3 = test_helpers::add_mixnode(
//             node_owner.as_str(),
//             coins(10_000_000_000 * 2, MIX_DENOM.base),
//             deps.as_mut(),
//         );
//
//         try_delegate_to_mixnode(
//             deps.as_mut(),
//             mock_env(),
//             mock_info("alicebob_d1", &[coin(8000_000000 * 2, MIX_DENOM.base)]),
//             node_identity_3.clone(),
//         )
//         .unwrap();
//
//         try_delegate_to_mixnode(
//             deps.as_mut(),
//             mock_env(),
//             mock_info("alicebob_d2", &[coin(2000_000000 * 2, MIX_DENOM.base)]),
//             node_identity_3.clone(),
//         )
//         .unwrap();
//
//         crate::delegations::transactions::_try_reconcile_all_delegation_events(
//             &mut deps.storage,
//             &deps.api,
//         )
//         .unwrap();
//
//         let info = mock_info(rewarding_validator_address.as_ref(), &[]);
//         env.block.height += 2 * constants::MINIMUM_BLOCK_AGE_FOR_REWARDING;
//
//         let mix_1 = mixnodes_storage::read_full_mixnode_bond(&deps.storage, &node_identity_1)
//             .unwrap()
//             .unwrap();
//         let mix_1_uptime = 100;
//
//         let mix_2 = mixnodes_storage::read_full_mixnode_bond(&deps.storage, &node_identity_2)
//             .unwrap()
//             .unwrap();
//         let mix_2_uptime = 50;
//
//         let mix_3 = mixnodes_storage::read_full_mixnode_bond(&deps.storage, &node_identity_3)
//             .unwrap()
//             .unwrap();
//
//         // average of 1 and 2
//         let mix_3_uptime = 75;
//
//         let epoch = Interval::init_epoch(env.clone());
//         save_epoch(&mut deps.storage, &epoch).unwrap();
//         save_epoch_reward_params(epoch.id(), &mut deps.storage).unwrap();
//
//         let interval_reward_params = current_epoch_reward_params(&deps.storage).unwrap();
//
//         let node_reward_params = NodeRewardParams::new(0, mix_1_uptime, true);
//         let node_reward_params_2 = NodeRewardParams::new(0, mix_2_uptime, true);
//         let node_reward_params_3 = NodeRewardParams::new(0, mix_3_uptime, true);
//
//         let mut params = RewardParams::new(interval_reward_params, node_reward_params);
//         let mut params2 = RewardParams::new(interval_reward_params, node_reward_params_2);
//         let mut params3 = RewardParams::new(interval_reward_params, node_reward_params_3);
//
//         params.set_reward_blockstamp(env.block.height);
//         params2.set_reward_blockstamp(env.block.height);
//         params3.set_reward_blockstamp(env.block.height);
//
//         assert_eq!(params.performance(), U128::from_num(1u32));
//         assert_eq!(params2.performance(), U128::from_num(0.5f32));
//         assert_eq!(params3.performance(), U128::from_num(0.75f32));
//
//         let mix_1_reward_result = mix_1.reward(&params);
//
//         let info = mock_info(rewarding_validator_address.as_str(), &[]);
//         crate::mixnodes::transactions::try_checkpoint_mixnodes(
//             &mut deps.storage,
//             env.block.height,
//             info.clone(),
//         )
//         .unwrap();
//         mixnodes
//             .assert_checkpointed(&deps.storage, env.block.height)
//             .unwrap();
//
//         try_reward_mixnode(
//             deps.as_mut(),
//             env.clone(),
//             info.clone(),
//             node_identity_1.clone(),
//             node_reward_params,
//         )
//         .unwrap();
//
//         let mix_after_reward = mixnodes.may_load(&deps.storage, &node_identity_1).unwrap();
//         let accumulated1 = mix_after_reward
//             .as_ref()
//             .unwrap()
//             .accumulated_rewards
//             .unwrap()
//             .u128();
//         // stupid one-off error, but @DU says it's fine
//         assert_eq!(accumulated1, 1948911 + 1);
//
//         let checkpoints = mixnodes
//             .changelog()
//             .prefix(&node_identity_1)
//             .keys(&deps.storage, None, None, Order::Ascending)
//             .filter_map(|x| x.ok())
//             .count();
//         assert_eq!(checkpoints, 2);
//
//         env.block.height += 10000;
//         env.block.time = env.block.time.plus_seconds(3601);
//
//         try_advance_epoch(
//             env.clone(),
//             &mut deps.storage,
//             rewarding_validator_address.to_string(),
//         )
//         .unwrap();
//
//         // After two snapshots we should see an increase in delegation
//         try_delegate_to_mixnode(
//             deps.as_mut(),
//             env.clone(),
//             mock_info("alice_d1", &[coin(8000_000000, MIX_DENOM.base)]),
//             node_identity_1.clone(),
//         )
//         .unwrap();
//
//         crate::delegations::transactions::_try_reconcile_all_delegation_events(
//             &mut deps.storage,
//             &deps.api,
//         )
//         .unwrap();
//
//         let info = mock_info(rewarding_validator_address.as_str(), &[]);
//         crate::mixnodes::transactions::try_checkpoint_mixnodes(
//             &mut deps.storage,
//             env.block.height,
//             info.clone(),
//         )
//         .unwrap();
//         mixnodes
//             .assert_checkpointed(&deps.storage, env.block.height)
//             .unwrap();
//
//         try_reward_mixnode(
//             deps.as_mut(),
//             env.clone(),
//             info,
//             node_identity_1.clone(),
//             node_reward_params,
//         )
//         .unwrap();
//         let mix_after_reward_2 = mixnodes.may_load(&deps.storage, &node_identity_1).unwrap();
//         let accumulated2 = mix_after_reward_2
//             .as_ref()
//             .unwrap()
//             .accumulated_rewards
//             .unwrap()
//             .u128();
//         assert_eq!(accumulated2, accumulated1 + 2728477);
//         assert_ne!(mix_after_reward, mix_after_reward_2);
//
//         let checkpoints = mixnodes
//             .changelog()
//             .prefix(&node_identity_1)
//             .keys(&deps.storage, None, None, Order::Ascending)
//             .collect::<Vec<Result<u64, StdError>>>();
//         assert_eq!(checkpoints.len(), 3);
//
//         env.block.height += 10000;
//         env.block.time = env.block.time.plus_seconds(3601);
//
//         try_advance_epoch(
//             env.clone(),
//             &mut deps.storage,
//             rewarding_validator_address.to_string(),
//         )
//         .unwrap();
//
//         let info = mock_info(rewarding_validator_address.as_str(), &[]);
//         crate::mixnodes::transactions::try_checkpoint_mixnodes(
//             &mut deps.storage,
//             env.block.height,
//             info.clone(),
//         )
//         .unwrap();
//         mixnodes
//             .assert_checkpointed(&deps.storage, env.block.height)
//             .unwrap();
//
//         try_reward_mixnode(
//             deps.as_mut(),
//             env.clone(),
//             info,
//             node_identity_1.clone(),
//             node_reward_params,
//         )
//         .unwrap();
//         let mix_after_reward_3 = mixnodes.may_load(&deps.storage, &node_identity_1).unwrap();
//         let accumulated3 = mix_after_reward_3
//             .as_ref()
//             .unwrap()
//             .accumulated_rewards
//             .unwrap()
//             .u128();
//         // off by one : )
//         assert_eq!(accumulated3 + 1, accumulated2 + 2728477);
//
//         let checkpoints = mixnodes
//             .changelog()
//             .prefix(&node_identity_1)
//             .keys(&deps.storage, None, None, Order::Ascending)
//             .filter_map(|x| x.ok())
//             .collect::<Vec<u64>>();
//         assert_eq!(checkpoints.len(), 4);
//
//         let delegation_map = crate::delegations::storage::delegations();
//         let key = "alice_d1".as_bytes().to_vec();
//
//         let last_claimed_height = storage::DELEGATOR_REWARD_CLAIMED_HEIGHT
//             .load(&deps.storage, (key.clone(), node_identity_1.to_string()))
//             .unwrap_or(0);
//
//         assert_eq!(last_claimed_height, 0);
//
//         let viable_delegations = delegation_map
//             .prefix((node_identity_1.to_string(), key.clone()))
//             .range(&deps.storage, None, None, Order::Descending)
//             .filter_map(|record| record.ok())
//             .filter(|(height, _)| last_claimed_height <= *height)
//             .map(|(_, delegation)| delegation)
//             .collect::<Vec<Delegation>>();
//
//         assert_eq!(viable_delegations.len(), 2);
//
//         let viable_heights = mixnodes
//             .changelog()
//             .prefix(&node_identity_1)
//             .keys(&deps.storage, None, None, Order::Ascending)
//             .filter_map(|height| height.ok())
//             .filter(|height| last_claimed_height <= *height)
//             .collect::<Vec<u64>>();
//
//         // Should be equal to the number of checkpoints
//         assert_eq!(viable_heights.len(), 4);
//
//         for (i, h) in viable_heights.into_iter().enumerate() {
//             let delegation_at_height = viable_delegations
//                 .iter()
//                 .filter(|d| d.block_height <= h)
//                 .fold(Uint128::zero(), |total, delegation| {
//                     total + delegation.amount.amount
//                 });
//             if i < 2 {
//                 assert_eq!(delegation_at_height, Uint128::new(8000000000));
//             } else {
//                 assert_eq!(delegation_at_height, Uint128::new(16000000000));
//             }
//         }
//
//         let alice_reward =
//             calculate_delegator_reward(&deps.storage, &deps.api, key.clone(), &node_identity_1)
//                 .unwrap();
//
//         // TODO: perform deeper investigation into this number as it seem to not have compounded
//         // reward on the initial 8000 delegation and only have done it starting from 16000
//         assert_eq!(alice_reward, Uint128::new(2737979));
//
//         let mix_0 = mixnodes.load(&deps.storage, &node_identity_1).unwrap();
//
//         _try_compound_delegator_reward(
//             env.block.height,
//             deps.as_mut(),
//             "alice_d1",
//             &node_identity_1,
//             None,
//         )
//         .unwrap();
//
//         crate::delegations::transactions::_try_reconcile_all_delegation_events(
//             &mut deps.storage,
//             &deps.api,
//         )
//         .unwrap();
//
//         let delegations = crate::delegations::storage::delegations()
//             .prefix((node_identity_1.to_string(), key.clone()))
//             .range(&deps.storage, None, None, Order::Ascending)
//             .filter_map(|x| x.ok())
//             .map(|(_, delegation)| delegation)
//             .collect::<Vec<Delegation>>();
//         assert_eq!(delegations.len(), 1);
//
//         let delegation = delegations.first().unwrap();
//         assert_eq!(
//             delegation.amount.amount,
//             Uint128::new(16000000000 + 2737979)
//         );
//
//         let mix_1 = mixnodes
//             .load(&deps.storage, &node_identity_1.clone())
//             .unwrap();
//
//         _try_remove_delegation_from_mixnode(deps.as_mut(), env, node_identity_1, "alice_d1", None)
//             .unwrap();
//
//         crate::delegations::transactions::_try_reconcile_all_delegation_events(
//             &mut deps.storage,
//             &deps.api,
//         )
//         .unwrap();
//
//         assert_eq!(
//             mix_0.accumulated_rewards(),
//             mix_1.accumulated_rewards() + alice_reward
//         );
//
//         let operator_reward =
//             calculate_operator_reward(&deps.storage, &deps.api, &Addr::unchecked("alice"), &mix_1)
//                 .unwrap();
//         assert_eq!(operator_reward, Uint128::new(2278901));
//
//         assert_eq!(mix_1_reward_result.sigma(), U128::from_num(0.0002f64));
//         assert_eq!(mix_1_reward_result.lambda(), U128::from_num(0.0001f64));
//         assert_eq!(mix_1_reward_result.reward().int(), accumulated1);
//
//         let mix_2_reward_result = mix_2.reward(&params2);
//
//         assert_eq!(mix_2_reward_result.sigma(), U128::from_num(0.0002f64));
//         assert_eq!(mix_2_reward_result.lambda(), U128::from_num(0.0001f64));
//         assert_eq!(mix_2_reward_result.reward().int(), 974456u128);
//
//         let mix_3_reward_result = mix_3.reward(&params3);
//
//         // assert_eq!(mix_3_reward_result.reward().int(), mix_1_reward_result.reward().int() + mix_2_reward_result.reward().int());
//     }
//
//     #[test]
//     fn test_tokenomics_rewarding() {
//         use crate::constants::INTERVAL_REWARD_PERCENT;
//         use crate::contract::INITIAL_REWARD_POOL;
//         use crate::support::helpers::epochs_in_interval;
//
//         type U128 = fixed::types::U75F53;
//
//         let mut deps = test_helpers::init_contract();
//         let mut env = mock_env();
//         let current_state = mixnet_params_storage::CONTRACT_STATE
//             .load(deps.as_ref().storage)
//             .unwrap();
//         let rewarding_validator_address = current_state.rewarding_validator_address;
//         let period_reward_pool =
//             (INITIAL_REWARD_POOL / 100 / epochs_in_interval(&deps.storage).unwrap() as u128)
//                 * INTERVAL_REWARD_PERCENT as u128;
//         assert_eq!(period_reward_pool, 6_944_444_444);
//         let circulating_supply = storage::circulating_supply(&deps.storage).unwrap().u128();
//         assert_eq!(circulating_supply, 750_000_000_000_000u128);
//
//         let node_owner: Addr = Addr::unchecked("alice");
//         let node_identity = test_helpers::add_mixnode(
//             node_owner.as_str(),
//             coins(10_000_000_000, MIX_DENOM.base),
//             deps.as_mut(),
//         );
//
//         try_delegate_to_mixnode(
//             deps.as_mut(),
//             mock_env(),
//             mock_info("alice_d1", &[coin(8000_000000, MIX_DENOM.base)]),
//             node_identity.clone(),
//         )
//         .unwrap();
//
//         try_delegate_to_mixnode(
//             deps.as_mut(),
//             mock_env(),
//             mock_info("alice_d2", &[coin(2000_000000, MIX_DENOM.base)]),
//             node_identity.clone(),
//         )
//         .unwrap();
//
//         crate::delegations::transactions::_try_reconcile_all_delegation_events(
//             &mut deps.storage,
//             &deps.api,
//         )
//         .unwrap();
//
//         let info = mock_info(rewarding_validator_address.as_ref(), &[]);
//         env.block.height += 2 * constants::MINIMUM_BLOCK_AGE_FOR_REWARDING;
//
//         let mix_1 = mixnodes_storage::read_full_mixnode_bond(&deps.storage, &node_identity)
//             .unwrap()
//             .unwrap();
//         let mix_1_uptime = 90;
//
//         let epoch = Interval::init_epoch(env.clone());
//         save_epoch(&mut deps.storage, &epoch).unwrap();
//         save_epoch_reward_params(epoch.id(), &mut deps.storage).unwrap();
//
//         let interval_reward_params = current_epoch_reward_params(&deps.storage).unwrap();
//
//         // =======================
//         // TODO: this is a temporary 'workaround' until we can produce "good" numbers for if the rewards
//         // were calculated based of 100M staking supply as opposed to original 750M circulating supply
//         let interval_reward_params = EpochRewardParams::new(
//             interval_reward_params.epoch_reward_pool(),
//             interval_reward_params.rewarded_set_size(),
//             interval_reward_params.active_set_size(),
//             circulating_supply,
//             interval_reward_params.sybil_resistance_percent(),
//             interval_reward_params.active_set_work_factor(),
//         );
//
//         // this one repeats internals of `save_epoch_reward_params` but with 750M staking supply as opposed to 100M
//         crate::interval::storage::CURRENT_EPOCH_REWARD_PARAMS
//             .save(&mut deps.storage, &interval_reward_params)
//             .unwrap();
//         crate::rewards::storage::EPOCH_REWARD_PARAMS
//             .save(&mut deps.storage, epoch.id(), &interval_reward_params)
//             .unwrap();
//         // =======================
//
//         let node_reward_params = NodeRewardParams::new(0, mix_1_uptime, true);
//
//         let mut params = RewardParams::new(interval_reward_params, node_reward_params);
//
//         params.set_reward_blockstamp(env.block.height);
//
//         assert_eq!(params.performance(), U128::from_num(0.8999999999999999f64));
//
//         let mix_1_reward_result = mix_1.reward(&params);
//
//         assert_eq!(
//             mix_1_reward_result.sigma(),
//             U128::from_num(0.0000266666666666f64)
//         );
//         assert_eq!(
//             mix_1_reward_result.lambda(),
//             U128::from_num(0.0000133333333333f64)
//         );
//         assert_eq!(mix_1_reward_result.reward().int(), 233202u128);
//
//         let base_operator_cost = current_operator_epoch_cost(&deps.storage).unwrap();
//
//         assert_eq!(
//             mix_1.node_profit(&params, base_operator_cost).int(),
//             183203u128
//         );
//
//         assert_ne!(
//             mix_1_reward_result.reward(),
//             mix_1.node_profit(&params, base_operator_cost).int()
//         );
//
//         let mix1_operator_reward = mix_1.operator_reward(&params, base_operator_cost);
//
//         let mix1_delegator1_reward =
//             mix_1.reward_delegation(Uint128::new(8000_000000), &params, base_operator_cost);
//
//         let mix1_delegator2_reward =
//             mix_1.reward_delegation(Uint128::new(2000_000000), &params, base_operator_cost);
//
//         assert_eq!(mix1_operator_reward, 150761);
//         assert_eq!(mix1_delegator1_reward, 65953);
//         assert_eq!(mix1_delegator2_reward, 16488);
//
//         assert_eq!(
//             mix_1_reward_result.reward().int(),
//             mix1_operator_reward + mix1_delegator1_reward + mix1_delegator2_reward
//         );
//
//         assert_eq!(
//             mix1_operator_reward + mix1_delegator1_reward + mix1_delegator2_reward,
//             mix_1_reward_result.reward().int()
//         );
//
//         let pre_reward_bond =
//             test_helpers::read_mixnode_pledge_amount(&deps.storage, &node_identity)
//                 .unwrap()
//                 .u128();
//         assert_eq!(pre_reward_bond, 10_000_000_000);
//
//         let pre_reward_delegation = mixnodes_storage::TOTAL_DELEGATION
//             .load(&deps.storage, &node_identity)
//             .unwrap()
//             .u128();
//         assert_eq!(pre_reward_delegation, 10_000_000_000);
//
//         try_reward_mixnode(
//             deps.as_mut(),
//             env,
//             info,
//             node_identity.clone(),
//             node_reward_params,
//         )
//         .unwrap();
//
//         let mixnode = crate::mixnodes::storage::mixnodes()
//             .load(&deps.storage, &node_identity)
//             .unwrap();
//
//         assert_eq!(
//             test_helpers::read_mixnode_pledge_amount(&deps.storage, &node_identity)
//                 .unwrap()
//                 .u128()
//                 + mixnode.accumulated_rewards().u128(),
//             pre_reward_bond
//                 + mix1_operator_reward
//                 + mix1_delegator1_reward
//                 + mix1_delegator2_reward
//         );
//
//         assert_eq!(
//             storage::REWARD_POOL.load(&deps.storage).unwrap().u128(),
//             INITIAL_REWARD_POOL
//                 - (mix1_operator_reward + mix1_delegator1_reward + mix1_delegator2_reward)
//         );
//
//         // it's all correctly saved
//         match storage::REWARDING_STATUS
//             .load(deps.as_ref().storage, (0u32, node_identity))
//             .unwrap()
//         {
//             RewardingStatus::Complete(result) => assert_eq!(
//                 RewardingResult {
//                     node_reward: Uint128::new(mix_1_reward_result.reward().checked_cast().unwrap()),
//                 },
//                 result
//             ),
//             _ => unreachable!(),
//         }
//     }
//
//     #[cfg(test)]
//     mod delegator_rewarding_tx {
//         use super::*;
//
//         #[test]
//         fn cannot_be_called_if_mixnode_is_fully_rewarded() {
//             // everything was done in a single reward call
//             let mut deps = test_helpers::init_contract();
//             let mut env = mock_env();
//             let current_state = mixnet_params_storage::CONTRACT_STATE
//                 .load(deps.as_mut().storage)
//                 .unwrap();
//             let rewarding_validator_address = current_state.rewarding_validator_address;
//
//             let node_owner: Addr = Addr::unchecked("alice");
//
//             #[allow(clippy::inconsistent_digit_grouping)]
//             let node_identity = test_helpers::add_mixnode(
//                 node_owner.as_str(),
//                 coins(10000_000_000, MIX_DENOM.base),
//                 deps.as_mut(),
//             );
//
//             env.block.height += constants::MINIMUM_BLOCK_AGE_FOR_REWARDING;
//
//             let epoch = Interval::init_epoch(env.clone());
//             save_epoch(&mut deps.storage, &epoch).unwrap();
//             save_epoch_reward_params(epoch.id(), &mut deps.storage).unwrap();
//
//             let info = mock_info(rewarding_validator_address.as_ref(), &[]);
//
//             try_reward_mixnode(
//                 deps.as_mut(),
//                 env.clone(),
//                 info.clone(),
//                 node_identity.clone(),
//                 tests::fixtures::node_reward_params_fixture(100),
//             )
//             .unwrap();
//
//             // there was another page of delegators, but they were already dealt with
//             let node_owner: Addr = Addr::unchecked("bob");
//
//             #[allow(clippy::inconsistent_digit_grouping)]
//             let node_identity = test_helpers::add_mixnode(
//                 node_owner.as_str(),
//                 coins(10000_000_000, MIX_DENOM.base),
//                 deps.as_mut(),
//             );
//
//             for i in 0..50 + 1 {
//                 try_delegate_to_mixnode(
//                     deps.as_mut(),
//                     env.clone(),
//                     mock_info(
//                         &*format!("delegator{:04}", i),
//                         &[coin(2000_000000, MIX_DENOM.base)],
//                     ),
//                     node_identity.clone(),
//                 )
//                 .unwrap();
//             }
//
//             env.block.height += constants::MINIMUM_BLOCK_AGE_FOR_REWARDING;
//             test_helpers::update_env_and_progress_interval(&mut env, deps.as_mut().storage);
//
//             let info = mock_info(rewarding_validator_address.as_ref(), &[]);
//             try_reward_mixnode(
//                 deps.as_mut(),
//                 env,
//                 info.clone(),
//                 node_identity.clone(),
//                 tests::fixtures::node_reward_params_fixture(100),
//             )
//             .unwrap();
//         }
//     }
// }

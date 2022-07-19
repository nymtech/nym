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
    ensure_bonded, ensure_is_authorized, ensure_is_owner, ensure_proxy_match,
    send_to_proxy_or_owner,
};
use cosmwasm_std::{wasm_execute, Addr, DepsMut, Env, MessageInfo, Response};
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
    _try_withdraw_operator_reward(deps, info.sender, None)
}

pub(crate) fn try_withdraw_operator_reward_on_behalf(
    deps: DepsMut<'_>,
    info: MessageInfo,
    owner: String,
) -> Result<Response, MixnetContractError> {
    let proxy = info.sender;
    let owner = deps.api.addr_validate(&owner)?;
    _try_withdraw_operator_reward(deps, owner, Some(proxy))
}

pub(crate) fn _try_withdraw_operator_reward(
    deps: DepsMut<'_>,
    owner: Addr,
    proxy: Option<Addr>,
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
            let track_reward_message = wasm_execute(proxy, &msg, vec![])?;
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
    _try_withdraw_delegator_reward(deps, mix_id, info.sender, None)
}

pub(crate) fn try_withdraw_delegator_reward_on_behalf(
    deps: DepsMut<'_>,
    info: MessageInfo,
    mix_id: NodeId,
    owner: String,
) -> Result<Response, MixnetContractError> {
    let proxy = info.sender;
    let owner = deps.api.addr_validate(&owner)?;
    _try_withdraw_delegator_reward(deps, mix_id, owner, Some(proxy))
}

pub(crate) fn _try_withdraw_delegator_reward(
    deps: DepsMut<'_>,
    mix_id: NodeId,
    owner: Addr,
    proxy: Option<Addr>,
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
            let track_reward_message = wasm_execute(proxy, &msg, vec![])?;
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
    ensure_is_owner(info.sender, deps.storage)?;

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
    ensure_is_owner(info.sender, deps.storage)?;

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

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::support::tests::test_helpers;
    use cosmwasm_std::testing::mock_info;

    #[cfg(test)]
    mod mixnode_rewarding {
        use super::*;
        use crate::interval::pending_events;
        use crate::support::tests::fixtures::TEST_COIN_DENOM;
        use crate::support::tests::test_helpers::{find_attribute, TestSetup};
        use cosmwasm_std::{Coin, Decimal, Uint128};
        use mixnet_contract_common::events::{
            MixnetEventType, BOND_NOT_FOUND_VALUE, DELEGATES_REWARD_KEY, NO_REWARD_REASON_KEY,
            OPERATOR_REWARD_KEY, ZERO_PERFORMANCE_VALUE,
        };
        use mixnet_contract_common::RewardedSetNodeStatus;

        #[test]
        fn can_only_be_performed_by_specified_rewarding_validator() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);
            let some_sender = mock_info("foomper", &[]);

            // skip time to when the following epoch is over (since mixnodes are not eligible for rewarding
            // in the same epoch they're bonded and we need the rewarding epoch to be over)
            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);
            let performance = test_helpers::performance(100.);

            let env = test.env();
            let res = try_reward_mixnode(test.deps_mut(), env, some_sender, mix_id, performance);
            assert_eq!(res, Err(MixnetContractError::Unauthorized));

            // good address (sanity check)
            let env = test.env();
            let sender = test.rewarding_validator();
            let res = try_reward_mixnode(test.deps_mut(), env, sender, mix_id, performance);
            assert!(res.is_ok());
        }

        #[test]
        fn can_only_be_performed_if_node_is_fully_bonded() {
            let mut test = TestSetup::new();
            let mix_id_never_existed = 42;
            let mix_id_unbonded = test.add_dummy_mixnode("mix-owner-unbonded", None);
            let mix_id_unbonded_leftover =
                test.add_dummy_mixnode("mix-owner-unbonded-leftover", None);
            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![
                mix_id_never_existed,
                mix_id_unbonded,
                mix_id_unbonded_leftover,
            ]);

            let env = test.env();

            // note: we don't have to test for cases where `is_unbonding` is set to true on a mixnode
            // since before performing the validator-api should clear out the event queue

            // manually adjust delegation info as to indicate the rewarding information shouldnt get removed
            let mut rewarding_details = storage::MIXNODE_REWARDING
                .load(test.deps().storage, mix_id_unbonded_leftover)
                .unwrap();
            rewarding_details.delegates = Decimal::raw(12345);
            rewarding_details.unique_delegations = 1;
            storage::MIXNODE_REWARDING
                .save(
                    test.deps_mut().storage,
                    mix_id_unbonded_leftover,
                    &rewarding_details,
                )
                .unwrap();
            pending_events::unbond_mixnode(test.deps_mut(), &env, mix_id_unbonded).unwrap();

            pending_events::unbond_mixnode(test.deps_mut(), &env, mix_id_unbonded_leftover)
                .unwrap();

            let env = test.env();
            let sender = test.rewarding_validator();
            let performance = test_helpers::performance(100.0);

            for &mix_id in &[
                mix_id_never_existed,
                mix_id_unbonded,
                mix_id_unbonded_leftover,
            ] {
                let res = try_reward_mixnode(
                    test.deps_mut(),
                    env.clone(),
                    sender.clone(),
                    mix_id,
                    performance,
                )
                .unwrap();

                let reason = find_attribute(
                    Some(MixnetEventType::MixnodeRewarding.to_string()),
                    NO_REWARD_REASON_KEY,
                    &res,
                );
                assert_eq!(BOND_NOT_FOUND_VALUE, reason);
            }
        }

        #[test]
        fn can_only_be_performed_once_epoch_is_over() {
            let mut test = TestSetup::new();

            let mix_id = test.add_dummy_mixnode("mix-owner", None);
            let sender = test.rewarding_validator();

            // node is in the active set BUT the current epoch has just begun
            test.skip_to_next_epoch();
            test.update_rewarded_set(vec![mix_id]);
            let performance = test_helpers::performance(100.);

            let env = test.env();
            let res = try_reward_mixnode(test.deps_mut(), env, sender.clone(), mix_id, performance);
            assert!(matches!(
                res,
                Err(MixnetContractError::EpochInProgress { .. })
            ));

            // epoch is over (sanity check)
            test.skip_to_current_epoch_end();
            let env = test.env();
            let res = try_reward_mixnode(test.deps_mut(), env, sender, mix_id, performance);
            assert!(res.is_ok());
        }

        #[test]
        fn can_only_be_performed_for_nodes_in_rewarded_set() {
            let mut test = TestSetup::new();

            let active_mix_id = test.add_dummy_mixnode("mix-owner-active", None);
            let standby_mix_id = test.add_dummy_mixnode("mix-owner-standby", None);
            let inactive_mix_id = test.add_dummy_mixnode("mix-owner-inactive", None);
            let sender = test.rewarding_validator();

            test.skip_to_next_epoch_end();

            // manually set the rewarded set so that we'd have 1 active node, 1 standby and 1 inactive
            interval_storage::REWARDED_SET
                .save(
                    test.deps_mut().storage,
                    active_mix_id,
                    &RewardedSetNodeStatus::Active,
                )
                .unwrap();
            interval_storage::REWARDED_SET
                .save(
                    test.deps_mut().storage,
                    standby_mix_id,
                    &RewardedSetNodeStatus::Standby,
                )
                .unwrap();

            let performance = test_helpers::performance(100.);
            let env = test.env();
            let res_active = try_reward_mixnode(
                test.deps_mut(),
                env.clone(),
                sender.clone(),
                active_mix_id,
                performance,
            );
            let res_standby = try_reward_mixnode(
                test.deps_mut(),
                env.clone(),
                sender.clone(),
                standby_mix_id,
                performance,
            );
            let res_inactive =
                try_reward_mixnode(test.deps_mut(), env, sender, inactive_mix_id, performance);

            assert!(res_active.is_ok());
            assert!(res_standby.is_ok());
            assert!(matches!(
                res_inactive,
                Err(MixnetContractError::MixnodeNotInRewardedSet { node_id, .. }) if node_id == inactive_mix_id
            ));
        }

        #[test]
        fn can_only_be_performed_once_per_node_per_epoch() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);
            let performance = test_helpers::performance(100.);
            let env = test.env();
            let sender = test.rewarding_validator();

            // first rewarding
            let res = try_reward_mixnode(
                test.deps_mut(),
                env.clone(),
                sender.clone(),
                mix_id,
                performance,
            );
            assert!(res.is_ok());

            // second rewarding
            let res = try_reward_mixnode(test.deps_mut(), env, sender.clone(), mix_id, performance);
            assert!(matches!(
                res,
                Err(MixnetContractError::MixnodeAlreadyRewarded { node_id, .. }) if node_id == mix_id
            ));

            // in the following epoch we're good again
            test.skip_to_next_epoch_end();
            let env = test.env();
            let res = try_reward_mixnode(test.deps_mut(), env, sender, mix_id, performance);
            assert!(res.is_ok());
        }

        #[test]
        fn requires_nonzero_performance_score() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);
            let zero_performance = test_helpers::performance(0.);
            let performance = test_helpers::performance(100.0);
            let env = test.env();
            let sender = test.rewarding_validator();

            // first rewarding
            let res = try_reward_mixnode(
                test.deps_mut(),
                env.clone(),
                sender.clone(),
                mix_id,
                zero_performance,
            )
            .unwrap();
            let reason = find_attribute(
                Some(MixnetEventType::MixnodeRewarding.to_string()),
                NO_REWARD_REASON_KEY,
                &res,
            );
            assert_eq!(ZERO_PERFORMANCE_VALUE, reason);

            // sanity check: it's still treated as rewarding, so you we can't reward the node again
            // with different performance for the same epoch
            let res = try_reward_mixnode(
                test.deps_mut(),
                env,
                sender.clone(),
                mix_id,
                zero_performance,
            );
            assert!(matches!(
                res,
                Err(MixnetContractError::MixnodeAlreadyRewarded { node_id, .. }) if node_id == mix_id
            ));

            // but in the next epoch, as always, we're good again
            test.skip_to_next_epoch_end();
            let env = test.env();
            let res =
                try_reward_mixnode(test.deps_mut(), env, sender, mix_id, performance).unwrap();

            // rewards got distributed (in this test we don't care what they were exactly, but they must be non-zero)
            let operator = find_attribute(
                Some(MixnetEventType::MixnodeRewarding.to_string()),
                OPERATOR_REWARD_KEY,
                &res,
            );
            assert!(!operator.is_empty());
            assert_ne!("0", operator);
            let delegates = find_attribute(
                Some(MixnetEventType::MixnodeRewarding.to_string()),
                DELEGATES_REWARD_KEY,
                &res,
            );
            assert_eq!("0", delegates);
        }

        #[test]
        fn correctly_accounts_for_rewards_distributed() {
            let mut test = TestSetup::new();
            let mix_id1 = test.add_dummy_mixnode("mix-owner1", None);
            let mix_id2 = test.add_dummy_mixnode("mix-owner2", None);
            let mix_id3 = test.add_dummy_mixnode("mix-owner3", None);

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id1, mix_id2, mix_id3]);
            let performance = test_helpers::performance(98.0);
            let env = test.env();
            let sender = test.rewarding_validator();

            test.add_delegation("delegator1", Uint128::new(100_000_000), mix_id2);

            test.add_delegation("delegator1", Uint128::new(100_000_000), mix_id3);
            test.add_delegation("delegator2", Uint128::new(123_456_000), mix_id3);
            test.add_delegation("delegator3", Uint128::new(9_100_000_000), mix_id3);

            let change = storage::PENDING_REWARD_POOL_CHANGE
                .load(test.deps().storage)
                .unwrap();
            assert!(change.removed.is_zero());
            assert!(change.added.is_zero());

            let mut total_operator = Decimal::zero();
            let mut total_delegates = Decimal::zero();

            for &mix_id in &[mix_id1, mix_id2, mix_id3] {
                let before = storage::MIXNODE_REWARDING
                    .load(test.deps().storage, mix_id)
                    .unwrap();

                let res = try_reward_mixnode(
                    test.deps_mut(),
                    env.clone(),
                    sender.clone(),
                    mix_id,
                    performance,
                )
                .unwrap();
                let operator: Decimal = find_attribute(
                    Some(MixnetEventType::MixnodeRewarding.to_string()),
                    OPERATOR_REWARD_KEY,
                    &res,
                )
                .parse()
                .unwrap();
                let delegates: Decimal = find_attribute(
                    Some(MixnetEventType::MixnodeRewarding.to_string()),
                    DELEGATES_REWARD_KEY,
                    &res,
                )
                .parse()
                .unwrap();

                let after = storage::MIXNODE_REWARDING
                    .load(test.deps().storage, mix_id)
                    .unwrap();

                // also the values emitted via events are consistent with the actual values!
                let actual_operator = after.operator - before.operator;
                let actual_delegates = after.delegates - before.delegates;
                assert_eq!(actual_operator, operator);
                assert_eq!(actual_delegates, delegates);

                total_operator += operator;
                total_delegates += delegates;

                let change = storage::PENDING_REWARD_POOL_CHANGE
                    .load(test.deps().storage)
                    .unwrap();
                assert_eq!(change.removed, total_operator + total_delegates);
                assert!(change.added.is_zero());
            }
        }

        #[test]
        fn correctly_splits_the_rewards() {
            // we're basing this test on our simulator that we have separate unit tests for
            // to determine whether values it spits are equal to what we expect

            let operator1 = Coin {
                amount: Uint128::new(100_000_000),
                denom: TEST_COIN_DENOM.into(),
            };
            let operator2 = Coin {
                amount: Uint128::new(2_570_000_000),
                denom: TEST_COIN_DENOM.into(),
            };
            let operator3 = Coin {
                amount: Uint128::new(12_345_000_000),
                denom: TEST_COIN_DENOM.into(),
            };
            let mut test = TestSetup::new();
            let mix_id1 = test.add_dummy_mixnode("mix-owner1", Some(operator1));
            let mix_id2 = test.add_dummy_mixnode("mix-owner2", Some(operator2));
            let mix_id3 = test.add_dummy_mixnode("mix-owner3", Some(operator3));

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id1, mix_id2, mix_id3]);
            let performance = test_helpers::performance(98.0);

            test.add_delegation("delegator1", Uint128::new(100_000_000), mix_id2);

            test.add_delegation("delegator1", Uint128::new(100_000_000), mix_id3);
            test.add_delegation("delegator2", Uint128::new(123_456_000), mix_id3);
            test.add_delegation("delegator3", Uint128::new(9_100_000_000), mix_id3);

            // repeat the rewarding the same set of delegates for few epochs
            for _ in 0..10 {
                for &mix_id in &[mix_id1, mix_id2, mix_id3] {
                    let mut sim = test.instantiate_simulator(mix_id);
                    let dist = test.reward_with_distribution(mix_id, performance);
                    let node_params = NodeRewardParams {
                        performance,
                        in_active_set: true,
                    };
                    let sim_res = sim.simulate_epoch(node_params);
                    assert_eq!(sim_res, dist);
                }
                test.skip_to_next_epoch_end();
            }

            // add few more delegations and repeat it
            // (note: we're not concerned about whether particular delegation owner got the correct amount,
            // this is checked in other unit tests)
            test.add_delegation("delegator1", Uint128::new(50_000_000), mix_id1);
            test.add_delegation("delegator1", Uint128::new(200_000_000), mix_id2);

            test.add_delegation("delegator5", Uint128::new(123_000_000), mix_id3);
            test.add_delegation("delegator6", Uint128::new(456_000_000), mix_id3);
            for _ in 0..10 {
                for &mix_id in &[mix_id1, mix_id2, mix_id3] {
                    let mut sim = test.instantiate_simulator(mix_id);
                    let dist = test.reward_with_distribution(mix_id, performance);
                    let node_params = NodeRewardParams {
                        performance,
                        in_active_set: true,
                    };
                    let sim_res = sim.simulate_epoch(node_params);
                    assert_eq!(sim_res, dist);
                }
                test.skip_to_next_epoch_end();
            }
        }
    }

    #[cfg(test)]
    mod withdrawing_operator_reward {
        use super::*;
    }

    #[cfg(test)]
    mod withdrawing_delegator_reward {
        use super::*;
    }

    #[cfg(test)]
    mod updating_active_set {
        use super::*;
    }

    #[cfg(test)]
    mod updating_rewarding_params {
        use super::*;
    }
}

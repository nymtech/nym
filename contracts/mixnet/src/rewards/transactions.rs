// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::delegations::storage as delegations_storage;
use crate::interval::storage as interval_storage;
use crate::interval::storage::{push_new_epoch_event, push_new_interval_event};
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::helpers::get_mixnode_details_by_owner;
use crate::mixnodes::storage as mixnodes_storage;
use crate::rewards::helpers;
use crate::support::helpers::{
    ensure_bonded, ensure_is_authorized, ensure_is_owner, ensure_proxy_match,
    send_to_proxy_or_owner,
};
use cosmwasm_std::{wasm_execute, Addr, DepsMut, Env, MessageInfo, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_active_set_update_event, new_mix_rewarding_event,
    new_not_found_mix_operator_rewarding_event, new_pending_active_set_update_event,
    new_pending_rewarding_params_update_event, new_rewarding_params_update_event,
    new_withdraw_delegator_reward_event, new_withdraw_operator_reward_event,
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
    let mix_id = mix_details.mix_id();

    ensure_proxy_match(&proxy, &mix_details.bond_information.proxy)?;
    ensure_bonded(&mix_details.bond_information)?;

    let reward = helpers::withdraw_operator_reward(deps.storage, mix_details)?;
    let mut response = Response::new();

    // if the reward is zero, don't track or send anything - there's no point
    if !reward.amount.is_zero() {
        let return_tokens = send_to_proxy_or_owner(&proxy, &owner, vec![reward.clone()]);
        response = response.add_message(return_tokens);

        if let Some(proxy) = &proxy {
            // we can only attempt to send the message to the vesting contract if the proxy IS the vesting contract
            // otherwise, we don't care
            let vesting_contract = mixnet_params_storage::vesting_contract_address(deps.storage)?;
            if proxy == &vesting_contract {
                // TODO: ask @DU if this is the intended way of using "TrackReward" (are we supposed
                // to use the same call for both operator and delegators?
                let msg = VestingContractExecuteMsg::TrackReward {
                    amount: reward.clone(),
                    address: owner.clone().into_string(),
                };
                let track_reward_message = wasm_execute(proxy, &msg, vec![])?;
                response = response.add_message(track_reward_message);
            }
        }
    }

    Ok(response.add_event(new_withdraw_operator_reward_event(
        &owner, &proxy, reward, mix_id,
    )))
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
        None => {
            return Err(MixnetContractError::NoMixnodeDelegationFound {
                mix_id,
                address: owner.into_string(),
                proxy: proxy.map(Addr::into_string),
            })
        }
        Some(delegation) => delegation,
    };

    // grab associated mixnode rewarding details
    let mix_rewarding =
        storage::MIXNODE_REWARDING.may_load(deps.storage, mix_id)?.ok_or(MixnetContractError::InconsistentState {
            comment: "mixnode rewarding got removed from the storage whilst there's still an existing delegation"
                .into(),
        })?;

    // see if the mixnode is not in the process of unbonding or whether it has already unbonded
    // (in that case the expected path of getting your tokens back is via undelegation)
    match mixnodes_storage::mixnode_bonds().may_load(deps.storage, mix_id)? {
        Some(mix_bond) if mix_bond.is_unbonding => {
            return Err(MixnetContractError::MixnodeIsUnbonding { node_id: mix_id })
        }
        None => return Err(MixnetContractError::MixnodeHasUnbonded { node_id: mix_id }),
        _ => (),
    };

    ensure_proxy_match(&proxy, &delegation.proxy)?;

    let reward = helpers::withdraw_delegator_reward(deps.storage, delegation, mix_rewarding)?;
    let mut response = Response::new();

    // if the reward is zero, don't track or send anything - there's no point
    if !reward.amount.is_zero() {
        let return_tokens = send_to_proxy_or_owner(&proxy, &owner, vec![reward.clone()]);
        response = response.add_message(return_tokens);

        if let Some(proxy) = &proxy {
            // we can only attempt to send the message to the vesting contract if the proxy IS the vesting contract
            // otherwise, we don't care
            let vesting_contract = mixnet_params_storage::vesting_contract_address(deps.storage)?;
            if proxy == &vesting_contract {
                // TODO: ask @DU if this is the intended way of using "TrackReward" (are we supposed
                // to use the same call for both operator and delegators?
                let msg = VestingContractExecuteMsg::TrackReward {
                    amount: reward.clone(),
                    address: owner.clone().into_string(),
                };
                let track_reward_message = wasm_execute(proxy, &msg, vec![])?;
                response = response.add_message(track_reward_message);
            }
        }
    }

    Ok(response.add_event(new_withdraw_delegator_reward_event(
        &owner, &proxy, reward, mix_id,
    )))
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
        Ok(Response::new().add_event(new_active_set_update_event(active_set_size)))
    } else {
        // push the epoch event
        let epoch_event = PendingEpochEvent::UpdateActiveSetSize {
            new_size: active_set_size,
        };
        push_new_epoch_event(deps.storage, &epoch_event)?;
        let time_left = interval.secs_until_current_interval_end(&env);
        Ok(
            Response::new().add_event(new_pending_active_set_update_event(
                active_set_size,
                time_left,
            )),
        )
    }
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
        Ok(Response::new().add_event(new_rewarding_params_update_event(
            updated_params,
            rewarding_params.interval,
        )))
    } else {
        // push the interval event
        let interval_event = PendingIntervalEvent::UpdateRewardingParams {
            update: updated_params,
        };
        push_new_interval_event(deps.storage, &interval_event)?;
        let time_left = interval.secs_until_current_interval_end(&env);
        Ok(
            Response::new().add_event(new_pending_rewarding_params_update_event(
                updated_params,
                time_left,
            )),
        )
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::mixnodes::storage as mixnodes_storage;
    use crate::support::tests::test_helpers;
    use cosmwasm_std::testing::mock_info;

    #[cfg(test)]
    mod mixnode_rewarding {
        use super::*;
        use crate::interval::pending_events;
        use crate::support::tests::test_helpers::{find_attribute, TestSetup};
        use cosmwasm_std::{Decimal, Uint128};
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

            let operator1 = Uint128::new(100_000_000);
            let operator2 = Uint128::new(2_570_000_000);
            let operator3 = Uint128::new(12_345_000_000);

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

            let performance = test_helpers::performance(12.3);
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
        use crate::interval::pending_events;
        use crate::support::tests::test_helpers::{assert_eq_with_leeway, TestSetup};
        use cosmwasm_std::{BankMsg, CosmosMsg, Decimal, Uint128};
        use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;

        #[test]
        fn can_only_be_done_if_delegation_exists() {
            let mut test = TestSetup::new();
            // add relatively huge stake so that the reward would be high enough to offset operating costs
            let mix_id1 =
                test.add_dummy_mixnode("mix-owner1", Some(Uint128::new(1_000_000_000_000)));
            let mix_id2 =
                test.add_dummy_mixnode("mix-owner2", Some(Uint128::new(1_000_000_000_000)));

            let delegator1 = "delegator1";
            let delegator2 = "delegator2";

            let sender1 = mock_info(delegator1, &[]);
            let sender2 = mock_info(delegator2, &[]);

            // note that there's no delegation from delegator1 towards mix1
            test.add_immediate_delegation(delegator2, 100_000_000u128, mix_id1);

            test.add_immediate_delegation(delegator1, 100_000_000u128, mix_id2);
            test.add_immediate_delegation(delegator2, 100_000_000u128, mix_id2);

            // perform some rewarding so that we'd have non-zero rewards
            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id1, mix_id2]);
            test.reward_with_distribution(mix_id1, test_helpers::performance(100.0));
            test.reward_with_distribution(mix_id2, test_helpers::performance(100.0));

            let res = try_withdraw_delegator_reward(test.deps_mut(), sender1.clone(), mix_id1);
            assert_eq!(
                res,
                Err(MixnetContractError::NoMixnodeDelegationFound {
                    mix_id: mix_id1,
                    address: delegator1.to_string(),
                    proxy: None,
                })
            );

            // sanity check for other ones
            let res = try_withdraw_delegator_reward(test.deps_mut(), sender1, mix_id2);
            assert!(res.is_ok());

            let res = try_withdraw_delegator_reward(test.deps_mut(), sender2.clone(), mix_id1);
            assert!(res.is_ok());

            let res = try_withdraw_delegator_reward(test.deps_mut(), sender2, mix_id2);
            assert!(res.is_ok());
        }

        #[test]
        fn tokens_are_only_sent_if_reward_is_nonzero() {
            let mut test = TestSetup::new();
            // add relatively huge stake so that the reward would be high enough to offset operating costs
            let mix_id1 =
                test.add_dummy_mixnode("mix-owner1", Some(Uint128::new(1_000_000_000_000)));
            let mix_id2 =
                test.add_dummy_mixnode("mix-owner2", Some(Uint128::new(1_000_000_000_000)));

            // very low stake so operating cost would be higher than total reward
            let low_stake_id =
                test.add_dummy_mixnode("mix-owner3", Some(Uint128::new(100_000_000)));

            let delegator = "delegator";
            let sender = mock_info(delegator, &[]);

            test.add_immediate_delegation(delegator, 100_000_000u128, mix_id1);
            test.add_immediate_delegation(delegator, 100_000_000u128, mix_id2);
            test.add_immediate_delegation(delegator, 1_000u128, low_stake_id);

            // reward mix1, but don't reward mix2
            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id1, low_stake_id]);
            test.reward_with_distribution(mix_id1, test_helpers::performance(100.0));
            test.reward_with_distribution(low_stake_id, test_helpers::performance(100.0));

            let res1 =
                try_withdraw_delegator_reward(test.deps_mut(), sender.clone(), mix_id1).unwrap();
            assert!(matches!(
                &res1.messages[0].msg,
                CosmosMsg::Bank(BankMsg::Send { to_address, amount }) if to_address == delegator && !amount[0].amount.is_zero()
            ),);

            let res2 =
                try_withdraw_delegator_reward(test.deps_mut(), sender.clone(), mix_id2).unwrap();
            assert!(res2.messages.is_empty());

            let res3 =
                try_withdraw_delegator_reward(test.deps_mut(), sender, low_stake_id).unwrap();
            assert!(res3.messages.is_empty());
        }

        #[test]
        fn can_only_be_done_for_fully_bonded_nodes() {
            // note: if node has unbonded or is in the process of unbonding, the expected
            // way of getting back the rewards is to completely undelegate
            let mut test = TestSetup::new();
            let mix_id_unbonding =
                test.add_dummy_mixnode("mix-owner1", Some(Uint128::new(1_000_000_000_000)));
            let mix_id_unbonded_leftover =
                test.add_dummy_mixnode("mix-owner2", Some(Uint128::new(1_000_000_000_000)));

            let delegator = "delegator";
            let sender = mock_info(delegator, &[]);

            test.add_immediate_delegation(delegator, 100_000_000u128, mix_id_unbonding);
            test.add_immediate_delegation(delegator, 100_000_000u128, mix_id_unbonded_leftover);

            let performance = test_helpers::performance(100.0);
            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id_unbonding, mix_id_unbonded_leftover]);

            // go through few rewarding cycles before unbonding nodes (partially or fully)
            for _ in 0..10 {
                test.reward_with_distribution(mix_id_unbonding, performance);
                test.reward_with_distribution(mix_id_unbonded_leftover, performance);

                test.skip_to_next_epoch_end();
            }

            // start unbonding the first node and fully unbond the other
            let mut bond = mixnodes_storage::mixnode_bonds()
                .load(test.deps().storage, mix_id_unbonding)
                .unwrap();
            bond.is_unbonding = true;
            mixnodes_storage::mixnode_bonds()
                .save(test.deps_mut().storage, mix_id_unbonding, &bond)
                .unwrap();

            let env = test.env();
            pending_events::unbond_mixnode(test.deps_mut(), &env, mix_id_unbonded_leftover)
                .unwrap();

            let res =
                try_withdraw_delegator_reward(test.deps_mut(), sender.clone(), mix_id_unbonding);
            assert_eq!(
                res,
                Err(MixnetContractError::MixnodeIsUnbonding {
                    node_id: mix_id_unbonding
                })
            );

            let res =
                try_withdraw_delegator_reward(test.deps_mut(), sender, mix_id_unbonded_leftover);
            assert_eq!(
                res,
                Err(MixnetContractError::MixnodeHasUnbonded {
                    node_id: mix_id_unbonded_leftover
                })
            );
        }

        #[test]
        fn correctly_determines_earned_share_and_resets_reward_ratio() {
            let mut test = TestSetup::new();
            let mix_id_single =
                test.add_dummy_mixnode("mix-owner1", Some(Uint128::new(1_000_000_000_000)));
            let mix_id_quad =
                test.add_dummy_mixnode("mix-owner2", Some(Uint128::new(1_000_000_000_000)));

            let delegator1 = "delegator1";
            let delegator2 = "delegator2";
            let delegator3 = "delegator3";
            let delegator4 = "delegator4";
            let sender1 = mock_info(delegator1, &[]);
            let sender2 = mock_info(delegator2, &[]);
            let sender3 = mock_info(delegator3, &[]);
            let sender4 = mock_info(delegator4, &[]);

            let amount_single = 100_000_000u128;

            let amount_quad1 = 50_000_000u128;
            let amount_quad2 = 200_000_000u128;
            let amount_quad3 = 250_000_000u128;
            let amount_quad4 = 500_000_000u128;

            test.add_immediate_delegation(delegator1, amount_single, mix_id_single);

            test.add_immediate_delegation(delegator1, amount_quad1, mix_id_quad);
            test.add_immediate_delegation(delegator2, amount_quad2, mix_id_quad);
            test.add_immediate_delegation(delegator3, amount_quad3, mix_id_quad);
            test.add_immediate_delegation(delegator4, amount_quad4, mix_id_quad);

            let performance = test_helpers::performance(100.0);
            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id_single, mix_id_quad]);

            // accumulate some rewards
            let mut accumulated_single = Decimal::zero();
            let mut accumulated_quad = Decimal::zero();
            for _ in 0..10 {
                let dist = test.reward_with_distribution(mix_id_single, performance);
                // sanity check to make sure test is actually doing what it's supposed to be doing
                assert!(!dist.delegates.is_zero());

                accumulated_single += dist.delegates;
                let dist = test.reward_with_distribution(mix_id_quad, performance);
                accumulated_quad += dist.delegates;

                test.skip_to_next_epoch_end();
            }

            let before = test.read_delegation(mix_id_single, delegator1, None);
            assert_eq!(before.cumulative_reward_ratio, Decimal::zero());
            let res1 =
                try_withdraw_delegator_reward(test.deps_mut(), sender1.clone(), mix_id_single)
                    .unwrap();
            let (_, reward) = test_helpers::get_bank_send_msg(&res1).unwrap();
            assert_eq!(truncate_reward_amount(accumulated_single), reward[0].amount);
            let after = test.read_delegation(mix_id_single, delegator1, None);
            assert_ne!(after.cumulative_reward_ratio, Decimal::zero());
            assert_eq!(
                after.cumulative_reward_ratio,
                test.mix_rewarding(mix_id_single).total_unit_reward
            );

            // withdraw first two rewards. note that due to scaling we expect second reward to be 4x the first one
            let before1 = test.read_delegation(mix_id_quad, delegator1, None);
            let before2 = test.read_delegation(mix_id_quad, delegator2, None);
            assert_eq!(before1.cumulative_reward_ratio, Decimal::zero());
            assert_eq!(before2.cumulative_reward_ratio, Decimal::zero());
            let res1 = try_withdraw_delegator_reward(test.deps_mut(), sender1.clone(), mix_id_quad)
                .unwrap();
            let (_, reward1) = test_helpers::get_bank_send_msg(&res1).unwrap();
            let res2 = try_withdraw_delegator_reward(test.deps_mut(), sender2.clone(), mix_id_quad)
                .unwrap();
            let (_, reward2) = test_helpers::get_bank_send_msg(&res2).unwrap();
            // the seeming "error" comes from reward truncation,
            // say "actual" reward1 was `100.9`, while reward2 was 4x that, i.e. `403.6
            // however, upon truncating and conversion to coins they'd be `100` and `403` respectively
            // (clearly no longer holding the 4x ratio exactly), but this is NOT a bug,
            // this is the expected behaviour.
            // what we must assert here is that |a - b| <= ratio, rather than just a == b
            // assert_eq!(reward1[0].amount * Uint128::new(4), reward2[0].amount);
            assert_eq_with_leeway(
                reward1[0].amount * Uint128::new(4),
                reward2[0].amount,
                Uint128::new(4),
            );

            let after1 = test.read_delegation(mix_id_quad, delegator1, None);
            assert_ne!(after1.cumulative_reward_ratio, Decimal::zero());
            assert_eq!(
                after1.cumulative_reward_ratio,
                test.mix_rewarding(mix_id_quad).total_unit_reward
            );
            let after2 = test.read_delegation(mix_id_quad, delegator2, None);
            assert_ne!(after2.cumulative_reward_ratio, Decimal::zero());
            assert_eq!(
                after2.cumulative_reward_ratio,
                test.mix_rewarding(mix_id_quad).total_unit_reward
            );

            // accumulate some more
            for _ in 0..10 {
                let dist = test.reward_with_distribution(mix_id_quad, performance);
                accumulated_quad += dist.delegates;
                test.skip_to_next_epoch_end();
            }

            let before1_new = test.read_delegation(mix_id_quad, delegator1, None);
            let before2_new = test.read_delegation(mix_id_quad, delegator2, None);
            let before3 = test.read_delegation(mix_id_quad, delegator3, None);
            let before4 = test.read_delegation(mix_id_quad, delegator4, None);

            assert_eq!(
                before1_new.cumulative_reward_ratio,
                after1.cumulative_reward_ratio
            );
            assert_eq!(
                before2_new.cumulative_reward_ratio,
                after2.cumulative_reward_ratio
            );
            assert_eq!(before3.cumulative_reward_ratio, Decimal::zero());
            assert_eq!(before4.cumulative_reward_ratio, Decimal::zero());

            let res1 =
                try_withdraw_delegator_reward(test.deps_mut(), sender1, mix_id_quad).unwrap();
            let (_, reward1_new) = test_helpers::get_bank_send_msg(&res1).unwrap();
            let res2 =
                try_withdraw_delegator_reward(test.deps_mut(), sender2, mix_id_quad).unwrap();
            let (_, reward2_new) = test_helpers::get_bank_send_msg(&res2).unwrap();

            // the ratio between first and second delegator is still the same
            assert_eq_with_leeway(
                reward1_new[0].amount * Uint128::new(4),
                reward2_new[0].amount,
                Uint128::new(4),
            );

            let res3 =
                try_withdraw_delegator_reward(test.deps_mut(), sender3, mix_id_quad).unwrap();
            let (_, reward3) = test_helpers::get_bank_send_msg(&res3).unwrap();
            let res4 =
                try_withdraw_delegator_reward(test.deps_mut(), sender4, mix_id_quad).unwrap();
            let (_, reward4) = test_helpers::get_bank_send_msg(&res4).unwrap();

            // (and so is the ratio between 3rd and 4th)
            assert_eq_with_leeway(
                reward3[0].amount * Uint128::new(2),
                reward4[0].amount,
                Uint128::new(2),
            );

            // and finally the total distributed equals to total reward claimed
            let total_claimed = reward1[0].amount
                + reward2[0].amount
                + reward1_new[0].amount
                + reward2_new[0].amount
                + reward3[0].amount
                + reward4[0].amount;

            // we're adding 6 values together so our leeway can be at most 6
            let accumulated_actual = truncate_reward_amount(accumulated_quad);
            assert_eq_with_leeway(total_claimed, accumulated_actual, Uint128::new(6));
        }
    }

    #[cfg(test)]
    mod withdrawing_delegator_reward {
        use super::*;
        use crate::interval::pending_events;
        use crate::support::tests::test_helpers::TestSetup;
        use cosmwasm_std::{BankMsg, CosmosMsg, Uint128};

        #[test]
        fn can_only_be_done_if_bond_exists() {
            let mut test = TestSetup::new();

            let owner = "mix-owner";
            let mix_id = test.add_dummy_mixnode(owner, Some(Uint128::new(1_000_000_000_000)));
            let sender = mock_info("random-guy", &[]);

            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id]);
            test.reward_with_distribution(mix_id, test_helpers::performance(100.0));

            let res = try_withdraw_operator_reward(test.deps_mut(), sender.clone());
            assert_eq!(
                res,
                Err(MixnetContractError::NoAssociatedMixNodeBond {
                    owner: sender.sender
                })
            )
        }

        #[test]
        fn tokens_are_only_sent_if_reward_is_nonzero() {
            let mut test = TestSetup::new();

            let owner1 = "mix-owner1";
            let owner2 = "mix-owner2";
            let mix_id1 = test.add_dummy_mixnode(owner1, Some(Uint128::new(1_000_000_000_000)));
            test.add_dummy_mixnode(owner2, Some(Uint128::new(1_000_000_000_000)));

            let sender1 = mock_info(owner1, &[]);
            let sender2 = mock_info(owner2, &[]);

            // reward mix1, but don't reward mix2
            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id1]);
            test.reward_with_distribution(mix_id1, test_helpers::performance(100.0));

            let res1 = try_withdraw_operator_reward(test.deps_mut(), sender1).unwrap();
            assert!(matches!(
                &res1.messages[0].msg,
                CosmosMsg::Bank(BankMsg::Send { to_address, amount }) if to_address == owner1 && !amount[0].amount.is_zero()
            ),);

            let res2 = try_withdraw_operator_reward(test.deps_mut(), sender2).unwrap();
            assert!(res2.messages.is_empty());
        }

        #[test]
        fn can_only_be_done_for_fully_bonded_nodes() {
            // note: if node has unbonded or is in the process of unbonding, the expected
            // way of getting back the rewards is finish the undelegation
            let mut test = TestSetup::new();
            let owner1 = "mix-owner1";
            let owner2 = "mix-owner2";
            let sender1 = mock_info(owner1, &[]);
            let sender2 = mock_info(owner2, &[]);
            let mix_id_unbonding =
                test.add_dummy_mixnode(owner1, Some(Uint128::new(1_000_000_000_000)));
            let mix_id_unbonded_leftover =
                test.add_dummy_mixnode(owner2, Some(Uint128::new(1_000_000_000_000)));

            // add some delegation to the second node so that it wouldn't be cleared upon unbonding
            test.add_immediate_delegation("delegator", 100_000_000u128, mix_id_unbonded_leftover);

            let performance = test_helpers::performance(100.0);
            test.skip_to_next_epoch_end();
            test.update_rewarded_set(vec![mix_id_unbonding, mix_id_unbonded_leftover]);

            // go through few rewarding cycles before unbonding nodes (partially or fully)
            for _ in 0..10 {
                test.reward_with_distribution(mix_id_unbonding, performance);
                test.reward_with_distribution(mix_id_unbonded_leftover, performance);

                test.skip_to_next_epoch_end();
            }

            // start unbonding the first node and fully unbond the other
            let mut bond = mixnodes_storage::mixnode_bonds()
                .load(test.deps().storage, mix_id_unbonding)
                .unwrap();
            bond.is_unbonding = true;
            mixnodes_storage::mixnode_bonds()
                .save(test.deps_mut().storage, mix_id_unbonding, &bond)
                .unwrap();

            let env = test.env();
            pending_events::unbond_mixnode(test.deps_mut(), &env, mix_id_unbonded_leftover)
                .unwrap();

            let res = try_withdraw_operator_reward(test.deps_mut(), sender1);
            assert_eq!(
                res,
                Err(MixnetContractError::MixnodeIsUnbonding {
                    node_id: mix_id_unbonding
                })
            );

            let res = try_withdraw_operator_reward(test.deps_mut(), sender2);
            assert_eq!(
                res,
                Err(MixnetContractError::NoAssociatedMixNodeBond {
                    owner: Addr::unchecked(owner2)
                })
            );
        }
    }

    #[cfg(test)]
    mod updating_active_set {
        use super::*;
        use crate::support::tests::test_helpers::TestSetup;

        #[test]
        fn can_only_be_done_by_contract_owner() {
            let mut test = TestSetup::new();

            let rewarding_validator = test.rewarding_validator();
            let owner = test.owner();
            let random = mock_info("random-guy", &[]);

            let env = test.env();
            let res = try_update_active_set_size(
                test.deps_mut(),
                env.clone(),
                rewarding_validator,
                42,
                false,
            );
            assert_eq!(res, Err(MixnetContractError::Unauthorized));

            let res = try_update_active_set_size(test.deps_mut(), env.clone(), random, 42, false);
            assert_eq!(res, Err(MixnetContractError::Unauthorized));

            let res = try_update_active_set_size(test.deps_mut(), env, owner, 42, false);
            assert!(res.is_ok())
        }

        #[test]
        fn new_size_cant_be_bigger_than_rewarded_set() {
            let mut test = TestSetup::new();
            let owner = test.owner();

            let rewarded_set_size = storage::REWARDING_PARAMS
                .load(test.deps().storage)
                .unwrap()
                .rewarded_set_size;

            let env = test.env();
            let res = try_update_active_set_size(
                test.deps_mut(),
                env,
                owner.clone(),
                rewarded_set_size + 1,
                false,
            );
            assert_eq!(res, Err(MixnetContractError::InvalidActiveSetSize));

            // if its equal, its fine
            // (make sure we start with the fresh state)
            let mut test = TestSetup::new();
            let env = test.env();
            let res = try_update_active_set_size(
                test.deps_mut(),
                env,
                owner.clone(),
                rewarded_set_size,
                false,
            );
            assert!(res.is_ok());

            // as well as if its any value lower than that
            let mut test = TestSetup::new();
            let env = test.env();
            let res = try_update_active_set_size(
                test.deps_mut(),
                env,
                owner,
                rewarded_set_size - 100,
                false,
            );
            assert!(res.is_ok());
        }

        #[test]
        fn if_interval_is_finished_change_happens_immediately() {
            let mut test = TestSetup::new();
            let owner = test.owner();

            let old = storage::REWARDING_PARAMS
                .load(test.deps().storage)
                .unwrap()
                .active_set_size;
            test.skip_to_current_interval_end();
            let env = test.env();
            let res = try_update_active_set_size(test.deps_mut(), env, owner.clone(), 42, false);
            assert!(res.is_ok());
            let new = storage::REWARDING_PARAMS
                .load(test.deps().storage)
                .unwrap()
                .active_set_size;
            assert_ne!(old, new);
            assert_eq!(new, 42);

            // sanity check for "normal" case
            let mut test = TestSetup::new();
            let env = test.env();
            let res = try_update_active_set_size(test.deps_mut(), env, owner, 42, false);
            assert!(res.is_ok());
            let new = storage::REWARDING_PARAMS
                .load(test.deps().storage)
                .unwrap()
                .active_set_size;
            assert_eq!(old, new);
        }

        #[test]
        fn if_update_is_forced_it_happens_immediately() {
            let mut test = TestSetup::new();
            let owner = test.owner();

            let old = storage::REWARDING_PARAMS
                .load(test.deps().storage)
                .unwrap()
                .active_set_size;
            let env = test.env();
            let res = try_update_active_set_size(test.deps_mut(), env, owner.clone(), 42, true);
            assert!(res.is_ok());
            let new = storage::REWARDING_PARAMS
                .load(test.deps().storage)
                .unwrap()
                .active_set_size;
            assert_ne!(old, new);
            assert_eq!(new, 42);
        }

        #[test]
        fn without_forcing_it_change_happens_upon_clearing_epoch_events() {
            let mut test = TestSetup::new();
            let owner = test.owner();

            let old = storage::REWARDING_PARAMS
                .load(test.deps().storage)
                .unwrap()
                .active_set_size;
            let env = test.env();
            let res = try_update_active_set_size(test.deps_mut(), env, owner, 42, false);
            assert!(res.is_ok());
            let new = storage::REWARDING_PARAMS
                .load(test.deps().storage)
                .unwrap()
                .active_set_size;
            assert_eq!(old, new);

            // make sure it's actually saved to pending events
            let events = test.pending_epoch_events();
            assert!(
                matches!(events[0], PendingEpochEvent::UpdateActiveSetSize { new_size } if new_size == 42)
            );

            test.execute_all_pending_events();
            let new = storage::REWARDING_PARAMS
                .load(test.deps().storage)
                .unwrap()
                .active_set_size;
            assert_ne!(old, new);
            assert_eq!(new, 42);
        }
    }

    #[cfg(test)]
    mod updating_rewarding_params {
        use super::*;
        use crate::support::tests::test_helpers::{assert_decimals, TestSetup};
        use cosmwasm_std::Decimal;

        #[test]
        fn can_only_be_done_by_contract_owner() {
            let mut test = TestSetup::new();

            let rewarding_validator = test.rewarding_validator();
            let owner = test.owner();
            let random = mock_info("random-guy", &[]);

            let update = IntervalRewardingParamsUpdate {
                reward_pool: None,
                staking_supply: None,
                sybil_resistance_percent: None,
                active_set_work_factor: None,
                interval_pool_emission: None,
                rewarded_set_size: Some(123),
            };

            let env = test.env();
            let res = try_update_rewarding_params(
                test.deps_mut(),
                env.clone(),
                rewarding_validator,
                update,
                false,
            );
            assert_eq!(res, Err(MixnetContractError::Unauthorized));

            let res =
                try_update_rewarding_params(test.deps_mut(), env.clone(), random, update, false);
            assert_eq!(res, Err(MixnetContractError::Unauthorized));

            let res = try_update_rewarding_params(test.deps_mut(), env, owner, update, false);
            assert!(res.is_ok())
        }

        #[test]
        fn request_must_contain_at_least_single_update() {
            let mut test = TestSetup::new();
            let owner = test.owner();

            let empty_update = IntervalRewardingParamsUpdate {
                reward_pool: None,
                staking_supply: None,
                sybil_resistance_percent: None,
                active_set_work_factor: None,
                interval_pool_emission: None,
                rewarded_set_size: None,
            };

            let env = test.env();
            let res = try_update_rewarding_params(test.deps_mut(), env, owner, empty_update, false);
            assert_eq!(res, Err(MixnetContractError::EmptyParamsChangeMsg));
        }

        #[test]
        fn if_interval_is_finished_change_happens_immediately() {
            let mut test = TestSetup::new();
            let owner = test.owner();

            let update = IntervalRewardingParamsUpdate {
                reward_pool: None,
                staking_supply: None,
                sybil_resistance_percent: None,
                active_set_work_factor: None,
                interval_pool_emission: None,
                rewarded_set_size: Some(123),
            };

            let old = storage::REWARDING_PARAMS.load(test.deps().storage).unwrap();
            test.skip_to_current_interval_end();

            let env = test.env();
            let res =
                try_update_rewarding_params(test.deps_mut(), env, owner.clone(), update, false);
            assert!(res.is_ok());
            let new = storage::REWARDING_PARAMS.load(test.deps().storage).unwrap();
            assert_ne!(old, new);
            assert_eq!(new.rewarded_set_size, 123);

            // sanity check for "normal" case
            let mut test = TestSetup::new();
            let env = test.env();
            let res = try_update_rewarding_params(test.deps_mut(), env, owner, update, false);
            assert!(res.is_ok());
            let new = storage::REWARDING_PARAMS.load(test.deps().storage).unwrap();
            assert_eq!(old, new);
        }

        #[test]
        fn if_update_is_forced_it_happens_immediately() {
            let mut test = TestSetup::new();
            let owner = test.owner();

            let update = IntervalRewardingParamsUpdate {
                reward_pool: None,
                staking_supply: None,
                sybil_resistance_percent: None,
                active_set_work_factor: None,
                interval_pool_emission: None,
                rewarded_set_size: Some(123),
            };

            let old = storage::REWARDING_PARAMS.load(test.deps().storage).unwrap();
            let env = test.env();
            let res =
                try_update_rewarding_params(test.deps_mut(), env, owner.clone(), update, true);
            assert!(res.is_ok());
            let new = storage::REWARDING_PARAMS.load(test.deps().storage).unwrap();
            assert_ne!(old, new);
            assert_eq!(new.rewarded_set_size, 123);
        }

        #[test]
        fn without_forcing_it_change_happens_upon_clearing_interval_events() {
            let mut test = TestSetup::new();
            let owner = test.owner();

            let update = IntervalRewardingParamsUpdate {
                reward_pool: None,
                staking_supply: None,
                sybil_resistance_percent: None,
                active_set_work_factor: None,
                interval_pool_emission: None,
                rewarded_set_size: Some(123),
            };

            let old = storage::REWARDING_PARAMS.load(test.deps().storage).unwrap();
            let env = test.env();
            let res = try_update_rewarding_params(test.deps_mut(), env, owner, update, false);
            assert!(res.is_ok());
            let new = storage::REWARDING_PARAMS.load(test.deps().storage).unwrap();
            assert_eq!(old, new);

            // make sure it's actually saved to pending events
            let events = test.pending_interval_events();
            assert!(
                matches!(events[0],PendingIntervalEvent::UpdateRewardingParams { update } if update.rewarded_set_size == Some(123))
            );

            test.execute_all_pending_events();
            let new = storage::REWARDING_PARAMS.load(test.deps().storage).unwrap();
            assert_ne!(old, new);
            assert_eq!(new.rewarded_set_size, 123);
        }

        #[test]
        fn upon_update_fields_are_recomputed_accordingly() {
            let mut test = TestSetup::new();
            let owner = test.owner();

            let old = storage::REWARDING_PARAMS.load(test.deps().storage).unwrap();

            let two = Decimal::from_atomics(2u32, 0).unwrap();
            let four = Decimal::from_atomics(4u32, 0).unwrap();

            // TODO: be more fuzzy about it and try to vary other fields that can cause
            // re-computation like pool emission or rewarded set size update
            let update = IntervalRewardingParamsUpdate {
                reward_pool: Some(old.interval.reward_pool / two),
                staking_supply: Some(old.interval.staking_supply * four),
                sybil_resistance_percent: None,
                active_set_work_factor: None,
                interval_pool_emission: None,
                rewarded_set_size: None,
            };

            let env = test.env();
            let res = try_update_rewarding_params(test.deps_mut(), env, owner, update, true);
            assert!(res.is_ok());
            let new = storage::REWARDING_PARAMS.load(test.deps().storage).unwrap();

            // with half the reward pool, our reward budget is also halved
            assert_decimals(
                old.interval.epoch_reward_budget,
                two * new.interval.epoch_reward_budget,
            );

            // and with 4x the staking supply, the saturation point is also increased 4-folds
            assert_decimals(
                four * old.interval.stake_saturation_point,
                new.interval.stake_saturation_point,
            );
        }
    }
}

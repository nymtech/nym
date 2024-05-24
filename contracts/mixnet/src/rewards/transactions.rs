// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::delegations::storage as delegations_storage;
use crate::interval::storage as interval_storage;
use crate::interval::storage::{push_new_epoch_event, push_new_interval_event};
use crate::mixnodes::helpers::get_mixnode_details_by_owner;
use crate::mixnodes::storage as mixnodes_storage;
use crate::rewards::helpers;
use crate::rewards::helpers::update_and_save_last_rewarded;
use crate::support::helpers::{
    ensure_bonded, ensure_can_advance_epoch, ensure_epoch_in_progress_state, ensure_is_owner,
    AttachSendTokens,
};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_active_set_update_event, new_mix_rewarding_event,
    new_not_found_mix_operator_rewarding_event, new_pending_active_set_update_event,
    new_pending_rewarding_params_update_event, new_rewarding_params_update_event,
    new_withdraw_delegator_reward_event, new_withdraw_operator_reward_event,
    new_zero_uptime_mix_operator_rewarding_event,
};
use mixnet_contract_common::pending_events::{PendingEpochEventKind, PendingIntervalEventKind};
use mixnet_contract_common::reward_params::{
    IntervalRewardingParamsUpdate, NodeRewardParams, Performance,
};
use mixnet_contract_common::{Delegation, EpochState, MixId};

pub(crate) fn try_reward_mixnode(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    mix_id: MixId,
    node_performance: Performance,
) -> Result<Response, MixnetContractError> {
    // check whether this `info.sender` is the same one as set in `epoch_status.being_advanced_by`
    // if so, return `epoch_status` so we could avoid having to perform extra read from the storage
    let current_epoch_status = ensure_can_advance_epoch(&info.sender, deps.storage)?;

    // see if the epoch has finished
    let interval = interval_storage::current_interval(deps.storage)?;
    if !interval.is_current_epoch_over(&env) {
        return Err(MixnetContractError::EpochInProgress {
            current_block_time: env.block.time.seconds(),
            epoch_start: interval.current_epoch_start_unix_timestamp(),
            epoch_end: interval.current_epoch_end_unix_timestamp(),
        });
    }
    let absolute_epoch_id = interval.current_epoch_absolute_id();

    if matches!(current_epoch_status.state, EpochState::Rewarding {last_rewarded, ..} if last_rewarded == mix_id)
    {
        return Err(MixnetContractError::MixnodeAlreadyRewarded {
            mix_id,
            absolute_epoch_id,
        });
    }

    // update the epoch state with this node as being rewarded most recently
    // (if the transaction fails down the line, it will be reverted)
    update_and_save_last_rewarded(deps.storage, current_epoch_status, mix_id)?;

    // there's a chance of this failing to load the details if the mixnode unbonded before rewards
    // were distributed and all of its delegators are also gone
    let mut mix_rewarding = match storage::MIXNODE_REWARDING.may_load(deps.storage, mix_id)? {
        Some(mix_rewarding) if mix_rewarding.still_bonded() => mix_rewarding,
        // don't fail if the node has unbonded as we don't want to fail the underlying transaction
        _ => {
            return Ok(Response::new()
                .add_event(new_not_found_mix_operator_rewarding_event(interval, mix_id)));
        }
    };

    let prior_delegates = mix_rewarding.delegates;
    let prior_unit_reward = mix_rewarding.full_reward_ratio();

    // check if this node has already been rewarded for the current epoch.
    // unlike the previous check, this one should be a hard error since this cannot be
    // influenced by users actions (note that previous epoch state checks should actually already guard us against it)
    if absolute_epoch_id == mix_rewarding.last_rewarded_epoch {
        return Err(MixnetContractError::MixnodeAlreadyRewarded {
            mix_id,
            absolute_epoch_id,
        });
    }

    // again a hard error since the rewarding validator should have known not to reward this node
    let node_status = interval_storage::REWARDED_SET
        .load(deps.storage, mix_id)
        .map_err(|_| MixnetContractError::MixnodeNotInRewardedSet {
            mix_id,
            absolute_epoch_id,
        })?;

    // no need to calculate anything as rewards are going to be 0 for everything
    // however, we still need to update last_rewarded_epoch field
    if node_performance.is_zero() {
        mix_rewarding.last_rewarded_epoch = absolute_epoch_id;
        storage::MIXNODE_REWARDING.save(deps.storage, mix_id, &mix_rewarding)?;
        return Ok(
            Response::new().add_event(new_zero_uptime_mix_operator_rewarding_event(
                interval, mix_id,
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
    mix_rewarding.distribute_rewards(reward_distribution, absolute_epoch_id);

    // persist changes happened to the storage
    storage::MIXNODE_REWARDING.save(deps.storage, mix_id, &mix_rewarding)?;
    storage::reward_accounting(deps.storage, node_reward)?;

    Ok(Response::new().add_event(new_mix_rewarding_event(
        interval,
        mix_id,
        reward_distribution,
        prior_delegates,
        prior_unit_reward,
    )))
}

pub(crate) fn try_withdraw_operator_reward(
    deps: DepsMut<'_>,
    info: MessageInfo,
) -> Result<Response, MixnetContractError> {
    // we need to grab all of the node's details, so we'd known original pledge alongside
    // all the earned rewards (and obviously to know if this node even exists and is still
    // in the bonded state)
    let mix_details = get_mixnode_details_by_owner(deps.storage, info.sender.clone())?.ok_or(
        MixnetContractError::NoAssociatedMixNodeBond {
            owner: info.sender.clone(),
        },
    )?;
    let mix_id = mix_details.mix_id();

    ensure_bonded(&mix_details.bond_information)?;

    let reward = helpers::withdraw_operator_reward(deps.storage, mix_details)?;
    let mut response = Response::new();

    // if the reward is zero, don't track or send anything - there's no point
    if !reward.amount.is_zero() {
        response = response.send_tokens(&info.sender, reward.clone())
    }

    Ok(response.add_event(new_withdraw_operator_reward_event(
        &info.sender,
        reward,
        mix_id,
    )))
}

pub(crate) fn try_withdraw_delegator_reward(
    deps: DepsMut<'_>,
    info: MessageInfo,
    mix_id: MixId,
) -> Result<Response, MixnetContractError> {
    // see if the delegation even exists
    let storage_key = Delegation::generate_storage_key(mix_id, &info.sender, None);
    let delegation = match delegations_storage::delegations().may_load(deps.storage, storage_key)? {
        None => {
            return Err(MixnetContractError::NoMixnodeDelegationFound {
                mix_id,
                address: info.sender.into_string(),
                proxy: None,
            });
        }
        Some(delegation) => delegation,
    };

    // grab associated mixnode rewarding details
    let mix_rewarding =
        storage::MIXNODE_REWARDING.may_load(deps.storage, mix_id)?.ok_or(MixnetContractError::inconsistent_state(
            "mixnode rewarding got removed from the storage whilst there's still an existing delegation"
        ))?;

    // see if the mixnode is not in the process of unbonding or whether it has already unbonded
    // (in that case the expected path of getting your tokens back is via undelegation)
    match mixnodes_storage::mixnode_bonds().may_load(deps.storage, mix_id)? {
        Some(mix_bond) if mix_bond.is_unbonding => {
            return Err(MixnetContractError::MixnodeIsUnbonding { mix_id });
        }
        None => return Err(MixnetContractError::MixnodeHasUnbonded { mix_id }),
        _ => (),
    };

    let reward = helpers::withdraw_delegator_reward(deps.storage, delegation, mix_rewarding)?;
    let mut response = Response::new();

    // if the reward is zero, don't track or send anything - there's no point
    if !reward.amount.is_zero() {
        response = response.send_tokens(&info.sender, reward.clone())
    }

    Ok(response.add_event(new_withdraw_delegator_reward_event(
        &info.sender,
        reward,
        mix_id,
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
        Ok(Response::new().add_event(new_active_set_update_event(
            env.block.height,
            active_set_size,
        )))
    } else {
        // updating active sety size is only allowed if the epoch is currently not in the process of being advanced
        // (unless the force flag was used)
        ensure_epoch_in_progress_state(deps.storage)?;

        // push the epoch event
        let epoch_event = PendingEpochEventKind::UpdateActiveSetSize {
            new_size: active_set_size,
        };
        push_new_epoch_event(deps.storage, &env, epoch_event)?;
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
            env.block.height,
            updated_params,
            rewarding_params.interval,
        )))
    } else {
        // changing rewarding parameters is only allowed if the epoch is currently not in the process of being advanced
        // (unless the force flag was used)
        ensure_epoch_in_progress_state(deps.storage)?;

        // push the interval event
        let interval_event = PendingIntervalEventKind::UpdateRewardingParams {
            update: updated_params,
        };
        push_new_interval_event(deps.storage, &env, interval_event)?;
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
    use cosmwasm_std::testing::mock_info;

    use crate::mixnodes::storage as mixnodes_storage;
    use crate::support::tests::test_helpers;

    use super::*;

    #[cfg(test)]
    mod mixnode_rewarding {
        use cosmwasm_std::{Decimal, Uint128};

        use mixnet_contract_common::events::{
            MixnetEventType, BOND_NOT_FOUND_VALUE, DELEGATES_REWARD_KEY, NO_REWARD_REASON_KEY,
            OPERATOR_REWARD_KEY, PRIOR_DELEGATES_KEY, PRIOR_UNIT_REWARD_KEY,
            ZERO_PERFORMANCE_VALUE,
        };
        use mixnet_contract_common::helpers::compare_decimals;
        use mixnet_contract_common::{EpochStatus, RewardedSetNodeStatus};

        use crate::interval::pending_events;
        use crate::support::tests::test_helpers::{find_attribute, TestSetup};

        use super::*;

        #[cfg(test)]
        mod epoch_state_is_correctly_updated {
            use super::*;

            #[test]
            fn when_target_mixnode_unbonded() {
                let mut test = TestSetup::new();
                let mix_id_unbonded = test.add_dummy_mixnode("mix-owner-unbonded", None);
                let mix_id_unbonded_leftover =
                    test.add_dummy_mixnode("mix-owner-unbonded-leftover", None);
                let mix_id_never_existed = 42;
                test.skip_to_next_epoch_end();
                test.force_change_rewarded_set(vec![
                    mix_id_unbonded,
                    mix_id_unbonded_leftover,
                    mix_id_never_existed,
                ]);
                test.start_epoch_transition();

                let env = test.env();

                // note: we don't have to test for cases where `is_unbonding` is set to true on a mixnode
                // since before performing the nym-api should clear out the event queue

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
                pending_events::unbond_mixnode(test.deps_mut(), &env, 123, mix_id_unbonded)
                    .unwrap();

                pending_events::unbond_mixnode(
                    test.deps_mut(),
                    &env,
                    123,
                    mix_id_unbonded_leftover,
                )
                .unwrap();

                let env = test.env();
                let sender = test.rewarding_validator();
                let performance = test_helpers::performance(100.0);

                try_reward_mixnode(
                    test.deps_mut(),
                    env.clone(),
                    sender.clone(),
                    mix_id_unbonded,
                    performance,
                )
                .unwrap();
                assert_eq!(
                    EpochState::Rewarding {
                        last_rewarded: mix_id_unbonded,
                        final_node_id: mix_id_never_existed,
                    },
                    interval_storage::current_epoch_status(test.deps().storage)
                        .unwrap()
                        .state
                );

                try_reward_mixnode(
                    test.deps_mut(),
                    env.clone(),
                    sender.clone(),
                    mix_id_unbonded_leftover,
                    performance,
                )
                .unwrap();
                assert_eq!(
                    EpochState::Rewarding {
                        last_rewarded: mix_id_unbonded_leftover,
                        final_node_id: mix_id_never_existed,
                    },
                    interval_storage::current_epoch_status(test.deps().storage)
                        .unwrap()
                        .state
                );

                try_reward_mixnode(
                    test.deps_mut(),
                    env,
                    sender,
                    mix_id_never_existed,
                    performance,
                )
                .unwrap();
                assert_eq!(
                    EpochState::ReconcilingEvents,
                    interval_storage::current_epoch_status(test.deps().storage)
                        .unwrap()
                        .state
                );
            }

            #[test]
            fn when_target_mixnode_has_zero_performance() {
                let mut test = TestSetup::new();
                let mix_id = test.add_dummy_mixnode("mix-owner", None);

                test.skip_to_next_epoch_end();
                test.force_change_rewarded_set(vec![mix_id]);
                test.start_epoch_transition();
                let zero_performance = test_helpers::performance(0.);
                let env = test.env();
                let sender = test.rewarding_validator();

                try_reward_mixnode(test.deps_mut(), env, sender, mix_id, zero_performance).unwrap();
                assert_eq!(
                    EpochState::ReconcilingEvents,
                    interval_storage::current_epoch_status(test.deps().storage)
                        .unwrap()
                        .state
                );
            }

            #[test]
            fn when_theres_only_one_node_to_reward() {
                let mut test = TestSetup::new();
                let mix_id = test.add_dummy_mixnode("mix-owner", None);

                test.skip_to_next_epoch_end();
                test.force_change_rewarded_set(vec![mix_id]);
                test.start_epoch_transition();
                let performance = test_helpers::performance(100.0);
                let env = test.env();
                let sender = test.rewarding_validator();

                try_reward_mixnode(test.deps_mut(), env, sender, mix_id, performance).unwrap();
                assert_eq!(
                    EpochState::ReconcilingEvents,
                    interval_storage::current_epoch_status(test.deps().storage)
                        .unwrap()
                        .state
                );
            }

            #[test]
            fn when_theres_multiple_nodes_to_reward() {
                let mut test = TestSetup::new();

                let mut ids = Vec::new();
                for i in 0..100 {
                    let mix_id = test.add_dummy_mixnode(&format!("mix-owner{i}"), None);
                    ids.push(mix_id);
                }

                test.skip_to_next_epoch_end();
                test.force_change_rewarded_set(ids.clone());
                test.start_epoch_transition();
                let performance = test_helpers::performance(100.0);
                let env = test.env();
                let sender = test.rewarding_validator();

                for mix_id in ids {
                    try_reward_mixnode(
                        test.deps_mut(),
                        env.clone(),
                        sender.clone(),
                        mix_id,
                        performance,
                    )
                    .unwrap();

                    let current_state = interval_storage::current_epoch_status(test.deps().storage)
                        .unwrap()
                        .state;
                    if mix_id == 100 {
                        assert_eq!(EpochState::ReconcilingEvents, current_state)
                    } else {
                        assert_eq!(
                            EpochState::Rewarding {
                                last_rewarded: mix_id,
                                final_node_id: 100,
                            },
                            current_state
                        )
                    }
                }
            }
        }

        #[test]
        fn can_only_be_performed_if_in_rewarding_state() {
            let bad_states = vec![
                EpochState::InProgress,
                EpochState::ReconcilingEvents,
                EpochState::AdvancingEpoch,
            ];

            for bad_state in bad_states {
                let mut test = TestSetup::new();
                let rewarding_validator = test.rewarding_validator();

                let mut status = EpochStatus::new(test.rewarding_validator().sender);
                status.state = bad_state;
                interval_storage::save_current_epoch_status(test.deps_mut().storage, &status)
                    .unwrap();

                test.skip_to_current_epoch_end();
                test.force_change_rewarded_set(vec![1, 2, 3]);
                let env = test.env();

                let res = try_reward_mixnode(
                    test.deps_mut(),
                    env,
                    rewarding_validator,
                    1,
                    test_helpers::performance(100.),
                );
                assert_eq!(
                    res,
                    Err(MixnetContractError::UnexpectedNonRewardingEpochState {
                        current_state: bad_state
                    })
                );
            }
        }

        #[test]
        fn can_only_be_performed_by_specified_rewarding_validator() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);
            let some_sender = mock_info("foomper", &[]);

            // skip time to when the following epoch is over (since mixnodes are not eligible for rewarding
            // in the same epoch they're bonded and we need the rewarding epoch to be over)
            test.skip_to_next_epoch_end();
            test.force_change_rewarded_set(vec![mix_id]);
            test.start_epoch_transition();
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
            test.force_change_rewarded_set(vec![
                mix_id_unbonded,
                mix_id_unbonded_leftover,
                mix_id_never_existed,
            ]);
            test.start_epoch_transition();

            let env = test.env();

            // note: we don't have to test for cases where `is_unbonding` is set to true on a mixnode
            // since before performing the nym-api should clear out the event queue

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
            pending_events::unbond_mixnode(test.deps_mut(), &env, 123, mix_id_unbonded).unwrap();

            pending_events::unbond_mixnode(test.deps_mut(), &env, 123, mix_id_unbonded_leftover)
                .unwrap();

            let env = test.env();
            let sender = test.rewarding_validator();
            let performance = test_helpers::performance(100.0);

            for &mix_id in &[
                mix_id_unbonded,
                mix_id_unbonded_leftover,
                mix_id_never_existed,
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
            test.force_change_rewarded_set(vec![mix_id]);
            let performance = test_helpers::performance(100.);

            let env = test.env();
            let res = try_reward_mixnode(test.deps_mut(), env, sender.clone(), mix_id, performance);
            assert!(matches!(
                res,
                Err(MixnetContractError::EpochInProgress { .. })
            ));

            // epoch is over (sanity check)
            test.skip_to_current_epoch_end();
            test.start_epoch_transition();
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

            // actually add one more dummy node with high id so we wouldn't go into the next state
            interval_storage::REWARDED_SET
                .save(
                    test.deps_mut().storage,
                    9001,
                    &RewardedSetNodeStatus::Standby,
                )
                .unwrap();
            test.start_epoch_transition();

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
                Err(MixnetContractError::MixnodeNotInRewardedSet { mix_id, .. }) if mix_id == inactive_mix_id
            ));
        }

        #[test]
        fn can_only_be_performed_once_per_node_per_epoch() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            test.skip_to_next_epoch_end();
            test.force_change_rewarded_set(vec![mix_id, 42]);
            test.start_epoch_transition();
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
                Err(MixnetContractError::MixnodeAlreadyRewarded { mix_id, .. }) if mix_id == mix_id
            ));

            // in the following epoch we're good again
            test.skip_to_next_epoch_end();
            test.start_epoch_transition();

            let env = test.env();
            let res = try_reward_mixnode(test.deps_mut(), env, sender, mix_id, performance);
            assert!(res.is_ok());
        }

        #[test]
        fn requires_nonzero_performance_score() {
            let mut test = TestSetup::new();
            let mix_id = test.add_dummy_mixnode("mix-owner", None);

            test.skip_to_next_epoch_end();
            test.force_change_rewarded_set(vec![mix_id, 42]);
            test.start_epoch_transition();
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
                Err(MixnetContractError::MixnodeAlreadyRewarded { mix_id, .. }) if mix_id == mix_id
            ));

            // but in the next epoch, as always, we're good again
            test.skip_to_next_epoch_end();
            test.start_epoch_transition();

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
            test.force_change_rewarded_set(vec![mix_id1, mix_id2, mix_id3]);
            test.start_epoch_transition();
            let performance = test_helpers::performance(98.0);
            let env = test.env();
            let sender = test.rewarding_validator();

            test.add_immediate_delegation("delegator1", Uint128::new(100_000_000), mix_id2);

            test.add_immediate_delegation("delegator1", Uint128::new(100_000_000), mix_id3);
            test.add_immediate_delegation("delegator2", Uint128::new(123_456_000), mix_id3);
            test.add_immediate_delegation("delegator3", Uint128::new(9_100_000_000), mix_id3);

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
            test.start_epoch_transition();
            test.force_change_rewarded_set(vec![mix_id1, mix_id2, mix_id3]);
            let performance = test_helpers::performance(98.0);

            test.add_immediate_delegation("delegator1", Uint128::new(100_000_000), mix_id2);

            test.add_immediate_delegation("delegator1", Uint128::new(100_000_000), mix_id3);
            test.add_immediate_delegation("delegator2", Uint128::new(123_456_000), mix_id3);
            test.add_immediate_delegation("delegator3", Uint128::new(9_100_000_000), mix_id3);

            // bypass proper epoch progression and force change the state
            test.set_epoch_in_progress_state();

            // repeat the rewarding the same set of delegates for few epochs
            for _ in 0..10 {
                test.start_epoch_transition();
                for &mix_id in &[mix_id1, mix_id2, mix_id3] {
                    let mut sim = test.instantiate_simulator(mix_id);
                    let dist = test.reward_with_distribution(mix_id, performance);
                    let node_params = NodeRewardParams {
                        performance,
                        in_active_set: true,
                    };
                    let sim_res = sim.simulate_epoch_single_node(node_params).unwrap();
                    assert_eq!(sim_res, dist);
                }
                // bypass proper epoch progression and force change the state
                test.set_epoch_in_progress_state();
                test.skip_to_next_epoch_end();
            }

            // add few more delegations and repeat it
            // (note: we're not concerned about whether particular delegation owner got the correct amount,
            // this is checked in other unit tests)
            test.add_immediate_delegation("delegator1", Uint128::new(50_000_000), mix_id1);
            test.add_immediate_delegation("delegator1", Uint128::new(200_000_000), mix_id2);

            test.add_immediate_delegation("delegator5", Uint128::new(123_000_000), mix_id3);
            test.add_immediate_delegation("delegator6", Uint128::new(456_000_000), mix_id3);

            // bypass proper epoch progression and force change the state
            test.set_epoch_in_progress_state();

            let performance = test_helpers::performance(12.3);
            for _ in 0..10 {
                test.start_epoch_transition();
                for &mix_id in &[mix_id1, mix_id2, mix_id3] {
                    let mut sim = test.instantiate_simulator(mix_id);
                    let dist = test.reward_with_distribution(mix_id, performance);
                    let node_params = NodeRewardParams {
                        performance,
                        in_active_set: true,
                    };
                    let sim_res = sim.simulate_epoch_single_node(node_params).unwrap();
                    assert_eq!(sim_res, dist);
                }
                // bypass proper epoch progression and force change the state
                test.set_epoch_in_progress_state();
                test.skip_to_next_epoch_end();
            }
        }

        #[test]
        fn emitted_event_attributes_allow_for_delegator_reward_recomputation() {
            let operator1 = Uint128::new(1_000_000_000);
            let operator2 = Uint128::new(12_345_000_000);

            let mut test = TestSetup::new();
            let sender = test.rewarding_validator();

            let mix_id1 = test.add_dummy_mixnode("mix-owner1", Some(operator1));
            let mix_id2 = test.add_dummy_mixnode("mix-owner2", Some(operator2));

            test.skip_to_next_epoch_end();
            test.force_change_rewarded_set(vec![mix_id1, mix_id2]);
            let performance = test_helpers::performance(98.0);

            test.add_immediate_delegation("delegator1", Uint128::new(100_000_000), mix_id1);
            test.add_immediate_delegation("delegator1", Uint128::new(100_000_000), mix_id2);

            test.add_immediate_delegation("delegator2", Uint128::new(123_456_000), mix_id1);

            let del11 = test.delegation(mix_id1, "delegator1", &None);
            let del12 = test.delegation(mix_id1, "delegator2", &None);
            let del21 = test.delegation(mix_id2, "delegator1", &None);

            for _ in 0..10 {
                test.start_epoch_transition();

                // we know from the previous tests that actual rewarding distribution matches the simulator
                let mut sim1 = test.instantiate_simulator(mix_id1);
                let mut sim2 = test.instantiate_simulator(mix_id2);

                let node_params = NodeRewardParams {
                    performance,
                    in_active_set: true,
                };

                let dist1 = sim1.simulate_epoch_single_node(node_params).unwrap();
                let dist2 = sim2.simulate_epoch_single_node(node_params).unwrap();

                let env = test.env();

                let actual_prior1 = test.mix_rewarding(mix_id1);
                let actual_prior2 = test.mix_rewarding(mix_id2);

                let res1 = try_reward_mixnode(
                    test.deps_mut(),
                    env.clone(),
                    sender.clone(),
                    mix_id1,
                    performance,
                )
                .unwrap();

                let prior_delegates1: Decimal = find_attribute(
                    Some(MixnetEventType::MixnodeRewarding),
                    PRIOR_DELEGATES_KEY,
                    &res1,
                )
                .parse()
                .unwrap();
                assert_eq!(prior_delegates1, actual_prior1.delegates);

                let delegates_reward1: Decimal = find_attribute(
                    Some(MixnetEventType::MixnodeRewarding),
                    DELEGATES_REWARD_KEY,
                    &res1,
                )
                .parse()
                .unwrap();
                assert_eq!(delegates_reward1, dist1.delegates);

                let prior_unit_reward: Decimal = find_attribute(
                    Some(MixnetEventType::MixnodeRewarding),
                    PRIOR_UNIT_REWARD_KEY,
                    &res1,
                )
                .parse()
                .unwrap();
                assert_eq!(actual_prior1.full_reward_ratio(), prior_unit_reward);

                // either use the constant for (which for now is the same for all nodes)
                // or query the contract for per-node value
                let unit_delegation_base = actual_prior1.unit_delegation;

                // recompute the state of fully compounded delegation from before this rewarding was distributed
                let pre_rewarding_del11 = del11.dec_amount().unwrap()
                    + (prior_unit_reward - del11.cumulative_reward_ratio)
                        * del11.dec_amount().unwrap()
                        / (del11.cumulative_reward_ratio + unit_delegation_base);

                let computed_del11_reward =
                    pre_rewarding_del11 / prior_delegates1 * delegates_reward1;

                let pre_rewarding_del12 = del12.dec_amount().unwrap()
                    + (prior_unit_reward - del12.cumulative_reward_ratio)
                        * del12.dec_amount().unwrap()
                        / (del12.cumulative_reward_ratio + unit_delegation_base);

                let computed_del12_reward =
                    pre_rewarding_del12 / prior_delegates1 * delegates_reward1;

                // sanity check
                compare_decimals(
                    computed_del11_reward + computed_del12_reward,
                    delegates_reward1,
                    None,
                );

                let res2 = try_reward_mixnode(
                    test.deps_mut(),
                    env.clone(),
                    sender.clone(),
                    mix_id2,
                    performance,
                )
                .unwrap();

                let prior_delegates2: Decimal = find_attribute(
                    Some(MixnetEventType::MixnodeRewarding),
                    PRIOR_DELEGATES_KEY,
                    &res2,
                )
                .parse()
                .unwrap();
                assert_eq!(prior_delegates2, actual_prior2.delegates);

                let delegates_reward2: Decimal = find_attribute(
                    Some(MixnetEventType::MixnodeRewarding),
                    DELEGATES_REWARD_KEY,
                    &res2,
                )
                .parse()
                .unwrap();
                assert_eq!(delegates_reward2, dist2.delegates);

                let prior_unit_reward: Decimal = find_attribute(
                    Some(MixnetEventType::MixnodeRewarding),
                    PRIOR_UNIT_REWARD_KEY,
                    &res2,
                )
                .parse()
                .unwrap();
                assert_eq!(actual_prior2.full_reward_ratio(), prior_unit_reward);

                // either use the constant for (which for now is the same for all nodes)
                // or query the contract for per-node value
                let unit_delegation_base = actual_prior2.unit_delegation;

                // recompute the state of fully compounded delegation from before this rewarding was distributed
                let pre_rewarding_del21 = del21.dec_amount().unwrap()
                    + (prior_unit_reward - del21.cumulative_reward_ratio)
                        * del21.dec_amount().unwrap()
                        / (del21.cumulative_reward_ratio + unit_delegation_base);

                let computed_del21_reward =
                    pre_rewarding_del21 / prior_delegates2 * delegates_reward2;

                assert_eq!(dist2.delegates, computed_del21_reward);

                test.skip_to_next_epoch_end();
                // bypass proper epoch progression and force change the state
                test.set_epoch_in_progress_state();
            }

            // add more delegations and check few more epochs (so that the delegations would start from non-default unit delegation value)
            test.add_immediate_delegation("delegator3", Uint128::new(15_850_000_000), mix_id1);
            test.add_immediate_delegation("delegator3", Uint128::new(15_850_000_000), mix_id2);

            let del13 = test.delegation(mix_id1, "delegator3", &None);
            let del23 = test.delegation(mix_id2, "delegator3", &None);

            for _ in 0..10 {
                test.start_epoch_transition();

                // we know from the previous tests that actual rewarding distribution matches the simulator
                let mut sim1 = test.instantiate_simulator(mix_id1);
                let mut sim2 = test.instantiate_simulator(mix_id2);

                let node_params = NodeRewardParams {
                    performance,
                    in_active_set: true,
                };

                let dist1 = sim1.simulate_epoch_single_node(node_params).unwrap();
                let dist2 = sim2.simulate_epoch_single_node(node_params).unwrap();

                let env = test.env();

                let actual_prior1 = test.mix_rewarding(mix_id1);
                let actual_prior2 = test.mix_rewarding(mix_id2);

                let res1 = try_reward_mixnode(
                    test.deps_mut(),
                    env.clone(),
                    sender.clone(),
                    mix_id1,
                    performance,
                )
                .unwrap();

                let prior_delegates1: Decimal = find_attribute(
                    Some(MixnetEventType::MixnodeRewarding),
                    PRIOR_DELEGATES_KEY,
                    &res1,
                )
                .parse()
                .unwrap();
                assert_eq!(prior_delegates1, actual_prior1.delegates);

                let delegates_reward1: Decimal = find_attribute(
                    Some(MixnetEventType::MixnodeRewarding),
                    DELEGATES_REWARD_KEY,
                    &res1,
                )
                .parse()
                .unwrap();
                assert_eq!(delegates_reward1, dist1.delegates);

                let prior_unit_reward: Decimal = find_attribute(
                    Some(MixnetEventType::MixnodeRewarding),
                    PRIOR_UNIT_REWARD_KEY,
                    &res1,
                )
                .parse()
                .unwrap();
                assert_eq!(actual_prior1.full_reward_ratio(), prior_unit_reward);

                // either use the constant for (which for now is the same for all nodes)
                // or query the contract for per-node value
                let unit_delegation_base = actual_prior1.unit_delegation;

                // recompute the state of fully compounded delegation from before this rewarding was distributed
                let pre_rewarding_del11 = del11.dec_amount().unwrap()
                    + (prior_unit_reward - del11.cumulative_reward_ratio)
                        * del11.dec_amount().unwrap()
                        / (del11.cumulative_reward_ratio + unit_delegation_base);

                let computed_del11_reward =
                    pre_rewarding_del11 / prior_delegates1 * delegates_reward1;

                let pre_rewarding_del12 = del12.dec_amount().unwrap()
                    + (prior_unit_reward - del12.cumulative_reward_ratio)
                        * del12.dec_amount().unwrap()
                        / (del12.cumulative_reward_ratio + unit_delegation_base);

                let computed_del12_reward =
                    pre_rewarding_del12 / prior_delegates1 * delegates_reward1;

                let pre_rewarding_del13 = del13.dec_amount().unwrap()
                    + (prior_unit_reward - del13.cumulative_reward_ratio)
                        * del13.dec_amount().unwrap()
                        / (del13.cumulative_reward_ratio + unit_delegation_base);

                let computed_del13_reward =
                    pre_rewarding_del13 / prior_delegates1 * delegates_reward1;

                // sanity check
                compare_decimals(
                    computed_del11_reward + computed_del12_reward + computed_del13_reward,
                    delegates_reward1,
                    None,
                );

                let res2 = try_reward_mixnode(
                    test.deps_mut(),
                    env.clone(),
                    sender.clone(),
                    mix_id2,
                    performance,
                )
                .unwrap();

                let prior_delegates2: Decimal = find_attribute(
                    Some(MixnetEventType::MixnodeRewarding),
                    PRIOR_DELEGATES_KEY,
                    &res2,
                )
                .parse()
                .unwrap();
                assert_eq!(prior_delegates2, actual_prior2.delegates);

                let delegates_reward2: Decimal = find_attribute(
                    Some(MixnetEventType::MixnodeRewarding),
                    DELEGATES_REWARD_KEY,
                    &res2,
                )
                .parse()
                .unwrap();
                assert_eq!(delegates_reward2, dist2.delegates);

                let prior_unit_reward: Decimal = find_attribute(
                    Some(MixnetEventType::MixnodeRewarding),
                    PRIOR_UNIT_REWARD_KEY,
                    &res2,
                )
                .parse()
                .unwrap();
                assert_eq!(actual_prior2.full_reward_ratio(), prior_unit_reward);

                // either use the constant for (which for now is the same for all nodes)
                // or query the contract for per-node value
                let unit_delegation_base = actual_prior2.unit_delegation;

                // recompute the state of fully compounded delegation from before this rewarding was distributed
                let pre_rewarding_del21 = del21.dec_amount().unwrap()
                    + (prior_unit_reward - del21.cumulative_reward_ratio)
                        * del21.dec_amount().unwrap()
                        / (del21.cumulative_reward_ratio + unit_delegation_base);

                let computed_del21_reward =
                    pre_rewarding_del21 / prior_delegates2 * delegates_reward2;

                let pre_rewarding_del23 = del23.dec_amount().unwrap()
                    + (prior_unit_reward - del23.cumulative_reward_ratio)
                        * del23.dec_amount().unwrap()
                        / (del23.cumulative_reward_ratio + unit_delegation_base);

                let computed_del23_reward =
                    pre_rewarding_del23 / prior_delegates2 * delegates_reward2;

                compare_decimals(
                    computed_del21_reward + computed_del23_reward,
                    delegates_reward2,
                    None,
                );

                test.skip_to_next_epoch_end();
                // bypass proper epoch progression and force change the state
                test.set_epoch_in_progress_state();
            }
        }
    }

    #[cfg(test)]
    mod withdrawing_delegator_reward {
        use crate::interval::pending_events;
        use crate::support::tests::test_helpers::{assert_eq_with_leeway, TestSetup};
        use cosmwasm_std::{BankMsg, CosmosMsg, Decimal, Uint128};
        use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;

        use super::*;

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
            test.force_change_rewarded_set(vec![mix_id1, mix_id2]);
            test.start_epoch_transition();
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
            test.force_change_rewarded_set(vec![mix_id1, low_stake_id]);
            test.start_epoch_transition();
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
            test.force_change_rewarded_set(vec![mix_id_unbonding, mix_id_unbonded_leftover]);

            // go through few rewarding cycles before unbonding nodes (partially or fully)
            for _ in 0..10 {
                test.start_epoch_transition();

                test.reward_with_distribution(mix_id_unbonding, performance);
                test.reward_with_distribution(mix_id_unbonded_leftover, performance);

                test.skip_to_next_epoch_end();
                // bypass proper epoch progression and force change the state
                test.set_epoch_in_progress_state();
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
            pending_events::unbond_mixnode(test.deps_mut(), &env, 123, mix_id_unbonded_leftover)
                .unwrap();

            let res =
                try_withdraw_delegator_reward(test.deps_mut(), sender.clone(), mix_id_unbonding);
            assert_eq!(
                res,
                Err(MixnetContractError::MixnodeIsUnbonding {
                    mix_id: mix_id_unbonding
                })
            );

            let res =
                try_withdraw_delegator_reward(test.deps_mut(), sender, mix_id_unbonded_leftover);
            assert_eq!(
                res,
                Err(MixnetContractError::MixnodeHasUnbonded {
                    mix_id: mix_id_unbonded_leftover
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
            test.force_change_rewarded_set(vec![mix_id_single, mix_id_quad]);

            // accumulate some rewards
            let mut accumulated_single = Decimal::zero();
            let mut accumulated_quad = Decimal::zero();
            for _ in 0..10 {
                test.start_epoch_transition();
                let dist = test.reward_with_distribution(mix_id_single, performance);
                // sanity check to make sure test is actually doing what it's supposed to be doing
                assert!(!dist.delegates.is_zero());

                accumulated_single += dist.delegates;
                let dist = test.reward_with_distribution(mix_id_quad, performance);
                accumulated_quad += dist.delegates;

                test.skip_to_next_epoch_end();
                // bypass proper epoch progression and force change the state
                test.set_epoch_in_progress_state();
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
                test.start_epoch_transition();

                let dist = test.reward_with_distribution(mix_id_quad, performance);
                accumulated_quad += dist.delegates;
                test.skip_to_next_epoch_end();
                // bypass proper epoch progression and force change the state
                test.set_epoch_in_progress_state();
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
    mod withdrawing_operator_reward {
        use super::*;
        use crate::interval::pending_events;
        use crate::support::tests::test_helpers::TestSetup;
        use cosmwasm_std::{Addr, BankMsg, CosmosMsg, Uint128};

        #[test]
        fn can_only_be_done_if_bond_exists() {
            let mut test = TestSetup::new();

            let owner = "mix-owner";
            let mix_id = test.add_dummy_mixnode(owner, Some(Uint128::new(1_000_000_000_000)));
            let sender = mock_info("random-guy", &[]);

            test.skip_to_next_epoch_end();
            test.force_change_rewarded_set(vec![mix_id]);
            test.start_epoch_transition();
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
            test.force_change_rewarded_set(vec![mix_id1]);
            test.start_epoch_transition();
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
            test.force_change_rewarded_set(vec![mix_id_unbonding, mix_id_unbonded_leftover]);

            // go through few rewarding cycles before unbonding nodes (partially or fully)
            for _ in 0..10 {
                test.start_epoch_transition();
                test.reward_with_distribution(mix_id_unbonding, performance);
                test.reward_with_distribution(mix_id_unbonded_leftover, performance);

                test.skip_to_next_epoch_end();
                // bypass proper epoch progression and force change the state
                test.set_epoch_in_progress_state();
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
            pending_events::unbond_mixnode(test.deps_mut(), &env, 123, mix_id_unbonded_leftover)
                .unwrap();

            let res = try_withdraw_operator_reward(test.deps_mut(), sender1);
            assert_eq!(
                res,
                Err(MixnetContractError::MixnodeIsUnbonding {
                    mix_id: mix_id_unbonding
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
        use mixnet_contract_common::EpochStatus;

        use crate::support::tests::test_helpers::TestSetup;

        use super::*;

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

                interval_storage::save_current_epoch_status(test.deps_mut().storage, &status)
                    .unwrap();

                let res = try_update_active_set_size(
                    test.deps_mut(),
                    env.clone(),
                    owner.clone(),
                    100,
                    false,
                );
                assert!(matches!(
                    res,
                    Err(MixnetContractError::EpochAdvancementInProgress { .. })
                ));

                let res_forced =
                    try_update_active_set_size(test.deps_mut(), env.clone(), owner, 100, true);
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
            let res = try_update_active_set_size(test.deps_mut(), env, owner, 42, true);
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
                matches!(events[0].kind, PendingEpochEventKind::UpdateActiveSetSize { new_size } if new_size == 42)
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
        use cosmwasm_std::Decimal;

        use mixnet_contract_common::EpochStatus;

        use crate::support::tests::test_helpers::{assert_decimals, TestSetup};

        use super::*;

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

            let update = IntervalRewardingParamsUpdate {
                reward_pool: None,
                staking_supply: None,
                staking_supply_scale_factor: None,
                sybil_resistance_percent: None,
                active_set_work_factor: None,
                interval_pool_emission: None,
                rewarded_set_size: Some(123),
            };

            for bad_state in bad_states {
                let mut test = TestSetup::new();
                let owner = test.owner();
                let env = test.env();

                let mut status = EpochStatus::new(test.rewarding_validator().sender);
                status.state = bad_state;

                interval_storage::save_current_epoch_status(test.deps_mut().storage, &status)
                    .unwrap();

                let res = try_update_rewarding_params(
                    test.deps_mut(),
                    env.clone(),
                    owner.clone(),
                    update,
                    false,
                );
                assert!(matches!(
                    res,
                    Err(MixnetContractError::EpochAdvancementInProgress { .. })
                ));

                let res_forced =
                    try_update_rewarding_params(test.deps_mut(), env.clone(), owner, update, true);
                assert!(res_forced.is_ok())
            }
        }

        #[test]
        fn can_only_be_done_by_contract_owner() {
            let mut test = TestSetup::new();

            let rewarding_validator = test.rewarding_validator();
            let owner = test.owner();
            let random = mock_info("random-guy", &[]);

            let update = IntervalRewardingParamsUpdate {
                reward_pool: None,
                staking_supply: None,
                staking_supply_scale_factor: None,
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
                staking_supply_scale_factor: None,
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
                staking_supply_scale_factor: None,
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
                staking_supply_scale_factor: None,
                sybil_resistance_percent: None,
                active_set_work_factor: None,
                interval_pool_emission: None,
                rewarded_set_size: Some(123),
            };

            let old = storage::REWARDING_PARAMS.load(test.deps().storage).unwrap();
            let env = test.env();
            let res = try_update_rewarding_params(test.deps_mut(), env, owner, update, true);
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
                staking_supply_scale_factor: None,
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
                matches!(events[0].kind,PendingIntervalEventKind::UpdateRewardingParams { update } if update.rewarded_set_size == Some(123))
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
                staking_supply_scale_factor: None,
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

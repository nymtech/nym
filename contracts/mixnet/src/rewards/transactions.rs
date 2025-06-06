// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::compat::helpers::ensure_can_withdraw_rewards;
use crate::delegations::storage as delegations_storage;
use crate::interval::storage as interval_storage;
use crate::interval::storage::{push_new_epoch_event, push_new_interval_event};
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnet_contract_settings::storage::ADMIN;
use crate::rewards::helpers;
use crate::rewards::helpers::update_and_save_last_rewarded;
use crate::rewards::storage::RewardingStorage;
use crate::support::helpers::{
    ensure_any_node_bonded, ensure_can_advance_epoch, ensure_epoch_in_progress_state,
};
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::events::{
    new_active_set_update_event, new_mix_rewarding_event,
    new_not_found_node_operator_rewarding_event, new_pending_active_set_update_event,
    new_pending_rewarding_params_update_event, new_rewarding_params_update_event,
    new_withdraw_delegator_reward_event, new_withdraw_operator_reward_event,
    new_zero_uptime_mix_operator_rewarding_event,
};
use mixnet_contract_common::pending_events::{PendingEpochEventKind, PendingIntervalEventKind};
use mixnet_contract_common::reward_params::{
    ActiveSetUpdate, IntervalRewardingParamsUpdate, NodeRewardingParameters,
};
use mixnet_contract_common::{Delegation, EpochState, MixNodeDetails, NodeId, NymNodeDetails};
use nym_contracts_common::helpers::ResponseExt;

pub(crate) fn try_reward_node(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    node_id: NodeId,
    node_rewarding_params: NodeRewardingParameters,
) -> Result<Response, MixnetContractError> {
    let rewarding_storage = RewardingStorage::load();

    // check whether this `info.sender` is the same one as set in `epoch_status.being_advanced_by`
    // if so, return `epoch_status` so we could avoid having to perform extra read from the storage
    let current_epoch_status = ensure_can_advance_epoch(&info.sender, deps.storage)?;

    // see if the epoch has finished
    let interval = interval_storage::current_interval(deps.storage)?;
    interval.ensure_current_epoch_is_over(&env)?;

    let absolute_epoch_id = interval.current_epoch_absolute_id();

    if let EpochState::Rewarding { last_rewarded, .. } = current_epoch_status.state {
        if last_rewarded >= node_id {
            return Err(MixnetContractError::NodeAlreadyRewarded {
                node_id,
                absolute_epoch_id,
            });
        }
    }

    // update the epoch state with this node as being rewarded most recently
    // (if the transaction fails down the line, this storage write will be reverted)
    update_and_save_last_rewarded(deps.storage, current_epoch_status, node_id)?;

    // there's a chance of this failing to load the details if the node unbonded before rewards
    // were distributed and all of its delegators are also gone

    // NOTE: legacy mixnode rewarding are stored under the same storage key
    // and have the same rewarding structure thus they'd also be loaded here
    let mut rewarding_info = match storage::NYMNODE_REWARDING.may_load(deps.storage, node_id)? {
        Some(rewarding_info) if rewarding_info.still_bonded() => rewarding_info,
        // don't fail if the node has unbonded (or it's a legacy gateway) as we don't want to fail the underlying transaction
        _ => {
            return Ok(
                Response::new().add_event(new_not_found_node_operator_rewarding_event(
                    interval, node_id,
                )),
            );
        }
    };

    let prior_delegates = rewarding_info.delegates;
    let prior_unit_reward = rewarding_info.full_reward_ratio();

    // check if this node has already been rewarded for the current epoch.
    // unlike the previous check, this one should be a hard error since this cannot be
    // influenced by users actions (note that previous epoch state checks should actually already guard us against it)
    if absolute_epoch_id == rewarding_info.last_rewarded_epoch {
        return Err(MixnetContractError::NodeAlreadyRewarded {
            node_id,
            absolute_epoch_id,
        });
    }

    // no need to calculate anything as rewards are going to be 0 for everything
    // however, we still need to update last_rewarded_epoch field
    if node_rewarding_params.is_zero() {
        rewarding_info.last_rewarded_epoch = absolute_epoch_id;
        storage::NYMNODE_REWARDING.save(deps.storage, node_id, &rewarding_info)?;
        return Ok(
            Response::new().add_event(new_zero_uptime_mix_operator_rewarding_event(
                interval, node_id,
            )),
        );
    }

    // make sure node's cost function is within the allowed range,
    // if not adjust it accordingly
    let params = mixnet_params_storage::state_params(deps.storage)?;
    let operator_params = params.operators_params;
    rewarding_info.normalise_cost_function(
        operator_params.profit_margin,
        operator_params.interval_operating_cost,
    );

    let global_rewarding_params = rewarding_storage
        .global_rewarding_params
        .load(deps.storage)?;

    // calculate each step separately for easier accounting
    //
    // total node reward, i.e. owner + delegates
    let node_reward = rewarding_info.node_reward(&global_rewarding_params, node_rewarding_params);

    // the actual split between owner and its delegates
    let reward_distribution = rewarding_info.determine_reward_split(
        node_reward,
        node_rewarding_params.performance,
        interval.epochs_in_interval(),
    );
    // update internal accounting with the new values
    rewarding_info.distribute_rewards(reward_distribution, absolute_epoch_id);

    // persist the changes to the storage
    rewarding_storage.try_persist_node_reward(
        deps.storage,
        node_id,
        rewarding_info,
        node_reward,
        node_rewarding_params.work_factor,
    )?;

    Ok(Response::new().add_event(new_mix_rewarding_event(
        interval,
        node_id,
        reward_distribution,
        prior_delegates,
        prior_unit_reward,
    )))
}

pub(crate) fn try_withdraw_nym_node_operator_reward(
    deps: DepsMut<'_>,
    node_details: NymNodeDetails,
) -> Result<Response, MixnetContractError> {
    let node_id = node_details.node_id();
    let owner = node_details.bond_information.owner.clone();

    ensure_can_withdraw_rewards(&node_details)?;

    let reward = helpers::withdraw_operator_reward(deps.storage, node_details)?;
    let mut response = Response::new();

    // if the reward is zero, don't track or send anything - there's no point
    if !reward.amount.is_zero() {
        response = response.send_tokens(&owner, reward.clone())
    }

    Ok(response.add_event(new_withdraw_operator_reward_event(&owner, reward, node_id)))
}

pub(crate) fn try_withdraw_mixnode_operator_reward(
    deps: DepsMut<'_>,
    mix_details: MixNodeDetails,
) -> Result<Response, MixnetContractError> {
    let node_id = mix_details.mix_id();
    let owner = mix_details.bond_information.owner.clone();

    ensure_can_withdraw_rewards(&mix_details)?;

    let reward = helpers::withdraw_operator_reward(deps.storage, mix_details)?;
    let mut response = Response::new();

    // if the reward is zero, don't track or send anything - there's no point
    if !reward.amount.is_zero() {
        response = response.send_tokens(&owner, reward.clone())
    }

    Ok(response.add_event(new_withdraw_operator_reward_event(&owner, reward, node_id)))
}

pub(crate) fn try_withdraw_delegator_reward(
    deps: DepsMut<'_>,
    info: MessageInfo,
    node_id: NodeId,
) -> Result<Response, MixnetContractError> {
    // see if the delegation even exists
    let storage_key = Delegation::generate_storage_key(node_id, &info.sender, None);
    let delegation = match delegations_storage::delegations().may_load(deps.storage, storage_key)? {
        None => {
            return Err(MixnetContractError::NodeDelegationNotFound {
                node_id,
                address: info.sender.into_string(),
                proxy: None,
            });
        }
        Some(delegation) => delegation,
    };

    // grab associated node rewarding details
    let mix_rewarding =
        storage::NYMNODE_REWARDING.may_load(deps.storage, node_id)?.ok_or(MixnetContractError::inconsistent_state(
            "nym-node/legacy mixnode rewarding got removed from the storage whilst there's still an existing delegation"
        ))?;

    // see if the mixnode is not in the process of unbonding or whether it has already unbonded
    // (in that case the expected path of getting your tokens back is via undelegation)
    ensure_any_node_bonded(deps.storage, node_id)?;

    let reward = helpers::withdraw_delegator_reward(deps.storage, delegation, mix_rewarding)?;
    let mut response = Response::new();

    // if the reward is zero, don't track or send anything - there's no point
    if !reward.amount.is_zero() {
        response = response.send_tokens(&info.sender, reward.clone())
    }

    Ok(response.add_event(new_withdraw_delegator_reward_event(
        &info.sender,
        reward,
        node_id,
    )))
}

pub(crate) fn try_update_active_set_distribution(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    update: ActiveSetUpdate,
    force_immediately: bool,
) -> Result<Response, MixnetContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

    let mut rewarding_params = RewardingStorage::load()
        .global_rewarding_params
        .load(deps.storage)?;

    // make sure the values could theoretically be applied in the current context
    rewarding_params.validate_active_set_update(update)?;

    let interval = interval_storage::current_interval(deps.storage)?;

    // perform the change immediately
    if force_immediately || interval.is_current_epoch_over(&env) {
        rewarding_params.try_change_active_set(update)?;
        storage::REWARDING_PARAMS.save(deps.storage, &rewarding_params)?;
        return Ok(Response::new().add_event(new_active_set_update_event(env.block.height, update)));
    }

    // otherwise push the event onto the queue to get executed when the epoch concludes

    // updating active set is only allowed if the epoch is currently not in the process of being advanced
    // (unless the force flag was used)
    ensure_epoch_in_progress_state(deps.storage)?;

    // push the epoch event
    let epoch_event = PendingEpochEventKind::UpdateActiveSet { update };
    push_new_epoch_event(deps.storage, &env, epoch_event)?;
    let time_left = interval.secs_until_current_interval_end(&env);
    Ok(Response::new().add_event(new_pending_active_set_update_event(update, time_left)))
}

pub(crate) fn try_update_rewarding_params(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    updated_params: IntervalRewardingParamsUpdate,
    force_immediately: bool,
) -> Result<Response, MixnetContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender)?;

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

#[allow(clippy::panic)]
#[allow(clippy::unreachable)]
#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::mixnodes::storage as mixnodes_storage;
    use crate::support::tests::fixtures::active_set_update_fixture;
    use crate::support::tests::test_helpers;
    use crate::support::tests::test_helpers::TestSetup;
    use cosmwasm_std::testing::message_info;

    // a simple wrapper to streamline checking for rewarding results
    trait TestRewarding {
        fn execute_rewarding(
            &mut self,
            node_id: NodeId,
            rewarding_params: NodeRewardingParameters,
        ) -> Result<Response, MixnetContractError>;

        fn assert_rewarding(
            &mut self,
            node_id: NodeId,
            rewarding_params: NodeRewardingParameters,
        ) -> Response;
    }

    impl TestRewarding for TestSetup {
        fn execute_rewarding(
            &mut self,
            node_id: NodeId,
            rewarding_params: NodeRewardingParameters,
        ) -> Result<Response, MixnetContractError> {
            let sender = self.rewarding_validator();
            self.execute_fn(
                |deps, env, info| try_reward_node(deps, env, info, node_id, rewarding_params),
                sender,
            )
        }

        #[track_caller]
        fn assert_rewarding(
            &mut self,
            node_id: NodeId,
            rewarding_params: NodeRewardingParameters,
        ) -> Response {
            let caller = std::panic::Location::caller();
            self.execute_rewarding(node_id, rewarding_params)
                .unwrap_or_else(|err| panic!("{caller} failed with: '{err}' ({err:?})"))
        }
    }

    #[cfg(test)]
    mod legacy_mixnode_rewarding {
        use super::*;
        use crate::interval::pending_events;
        use crate::support::tests::test_helpers::{find_attribute, FindAttribute, TestSetup};
        use cosmwasm_std::{Decimal, Uint128};
        use mixnet_contract_common::events::{
            MixnetEventType, BOND_NOT_FOUND_VALUE, DELEGATES_REWARD_KEY, NO_REWARD_REASON_KEY,
            OPERATOR_REWARD_KEY, PRIOR_DELEGATES_KEY, PRIOR_UNIT_REWARD_KEY,
            ZERO_PERFORMANCE_OR_WORK_VALUE,
        };
        use mixnet_contract_common::helpers::compare_decimals;
        use mixnet_contract_common::nym_node::Role;
        use mixnet_contract_common::reward_params::WorkFactor;
        use mixnet_contract_common::EpochStatus;

        #[cfg(test)]
        mod epoch_state_is_correctly_updated {
            use super::*;
            use mixnet_contract_common::reward_params::WorkFactor;

            #[test]
            fn when_target_mixnode_unbonded() {
                let mut test = TestSetup::new();
                let node_id_unbonded =
                    test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner-unbonded"), None);
                let node_id_unbonded_leftover = test.add_rewarded_legacy_mixnode(
                    &test.make_addr("mix-owner-unbonded-leftover"),
                    None,
                );
                let node_id_never_existed = 42;
                test.skip_to_next_epoch_end();
                test.force_change_mix_rewarded_set(vec![
                    node_id_unbonded,
                    node_id_unbonded_leftover,
                    node_id_never_existed,
                ]);
                test.start_epoch_transition();
                let active_params = test.active_node_params(100.);

                let env = test.env();

                // note: we don't have to test for cases where `is_unbonding` is set to true on a mixnode
                // since before performing the nym-api should clear out the event queue

                // manually adjust delegation info as to indicate the rewarding information shouldnt get removed
                let mut rewarding_details = storage::NYMNODE_REWARDING
                    .load(test.deps().storage, node_id_unbonded_leftover)
                    .unwrap();
                rewarding_details.delegates = Decimal::raw(12345);
                rewarding_details.unique_delegations = 1;
                storage::NYMNODE_REWARDING
                    .save(
                        test.deps_mut().storage,
                        node_id_unbonded_leftover,
                        &rewarding_details,
                    )
                    .unwrap();
                pending_events::unbond_mixnode(test.deps_mut(), &env, 123, node_id_unbonded)
                    .unwrap();

                pending_events::unbond_mixnode(
                    test.deps_mut(),
                    &env,
                    123,
                    node_id_unbonded_leftover,
                )
                .unwrap();

                test.assert_rewarding(node_id_unbonded, active_params);
                assert_eq!(
                    EpochState::Rewarding {
                        last_rewarded: node_id_unbonded,
                        final_node_id: node_id_never_existed,
                    },
                    interval_storage::current_epoch_status(test.deps().storage)
                        .unwrap()
                        .state
                );

                test.assert_rewarding(node_id_unbonded_leftover, active_params);
                assert_eq!(
                    EpochState::Rewarding {
                        last_rewarded: node_id_unbonded_leftover,
                        final_node_id: node_id_never_existed,
                    },
                    interval_storage::current_epoch_status(test.deps().storage)
                        .unwrap()
                        .state
                );

                test.assert_rewarding(node_id_never_existed, active_params);
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
                let node_id = test.add_rewarded_legacy_mixnode(
                    &test.make_addr(test.make_addr("mix-owner")),
                    None,
                );

                test.skip_to_next_epoch_end();
                test.force_change_mix_rewarded_set(vec![node_id]);
                test.start_epoch_transition();

                let zero_performance = test.active_node_params(0.);
                test.assert_rewarding(node_id, zero_performance);
                assert_eq!(
                    EpochState::ReconcilingEvents,
                    interval_storage::current_epoch_status(test.deps().storage)
                        .unwrap()
                        .state
                );
            }

            #[test]
            fn when_target_mixnode_has_zero_work_factor() {
                let mut test = TestSetup::new();
                let node_id = test.add_rewarded_legacy_mixnode(
                    &test.make_addr(test.make_addr("mix-owner")),
                    None,
                );

                test.skip_to_next_epoch_end();
                test.force_change_mix_rewarded_set(vec![node_id]);
                test.start_epoch_transition();

                let params = NodeRewardingParameters::new(
                    test_helpers::performance(100.),
                    WorkFactor::zero(),
                );
                test.assert_rewarding(node_id, params);
                assert_eq!(
                    EpochState::ReconcilingEvents,
                    interval_storage::current_epoch_status(test.deps().storage)
                        .unwrap()
                        .state
                );
            }

            #[test]
            fn when_target_nymnode_has_zero_performance() {
                let mut test = TestSetup::new();
                let node_id = test.add_dummy_nymnode(&test.make_addr("node-owner"), None);

                test.skip_to_next_epoch_end();
                test.force_change_mix_rewarded_set(vec![node_id]);
                test.start_epoch_transition();

                let zero_performance = test.active_node_params(0.);
                test.assert_rewarding(node_id, zero_performance);
                assert_eq!(
                    EpochState::ReconcilingEvents,
                    interval_storage::current_epoch_status(test.deps().storage)
                        .unwrap()
                        .state
                );
            }

            #[test]
            fn when_target_node_has_zero_workfactor() {
                let mut test = TestSetup::new();
                let node_id =
                    test.add_dummy_nymnode(&test.make_addr(test.make_addr("mix-owner")), None);

                test.skip_to_next_epoch_end();
                test.force_change_mix_rewarded_set(vec![node_id]);
                test.start_epoch_transition();

                let params = NodeRewardingParameters::new(
                    test_helpers::performance(100.),
                    WorkFactor::zero(),
                );
                test.assert_rewarding(node_id, params);
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
                let node_id = test.add_rewarded_legacy_mixnode(
                    &test.make_addr(test.make_addr("mix-owner")),
                    None,
                );

                test.skip_to_next_epoch_end();
                test.force_change_mix_rewarded_set(vec![node_id]);
                test.start_epoch_transition();
                let active_params = test.active_node_params(100.);

                test.assert_rewarding(node_id, active_params);
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
                    let node_id = test.add_rewarded_legacy_mixnode(
                        &test.make_addr(test.make_addr(format!("mix-owner{i}"))),
                        None,
                    );
                    ids.push(node_id);
                }

                test.skip_to_next_epoch_end();
                test.force_change_mix_rewarded_set(ids.clone());
                test.start_epoch_transition();
                let active_params = test.active_node_params(100.);

                for node_id in ids {
                    test.assert_rewarding(node_id, active_params);

                    let current_state = interval_storage::current_epoch_status(test.deps().storage)
                        .unwrap()
                        .state;
                    if node_id == 100 {
                        assert_eq!(EpochState::ReconcilingEvents, current_state)
                    } else {
                        assert_eq!(
                            EpochState::Rewarding {
                                last_rewarded: node_id,
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
                EpochState::RoleAssignment {
                    next: Role::first(),
                },
            ];

            for bad_state in bad_states {
                let mut test = TestSetup::new();
                let active_params = test.active_node_params(100.);

                let mut status = EpochStatus::new(test.rewarding_validator().sender);
                status.state = bad_state;
                interval_storage::save_current_epoch_status(test.deps_mut().storage, &status)
                    .unwrap();

                test.skip_to_current_epoch_end();
                test.force_change_mix_rewarded_set(vec![1, 2, 3]);

                let res = test.execute_rewarding(1, active_params);

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
            let node_id = test
                .add_rewarded_legacy_mixnode(&test.make_addr(test.make_addr("mix-owner")), None);
            let some_sender = message_info(&test.make_addr("foomper"), &[]);

            // skip time to when the following epoch is over (since mixnodes are not eligible for rewarding
            // in the same epoch they're bonded and we need the rewarding epoch to be over)
            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![node_id]);
            test.start_epoch_transition();
            let params = test.legacy_rewarding_params(node_id, 100.);

            let env = test.env();
            let res = try_reward_node(test.deps_mut(), env, some_sender, node_id, params);
            assert_eq!(res, Err(MixnetContractError::Unauthorized));

            // good address (sanity check)
            let env = test.env();
            let sender = test.rewarding_validator();
            let res = try_reward_node(test.deps_mut(), env, sender, node_id, params);
            assert!(res.is_ok());
        }

        #[test]
        fn can_only_be_performed_if_node_is_fully_bonded() {
            let mut test = TestSetup::new();
            let node_id_never_existed = 42;
            let node_id_unbonded =
                test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner-unbonded"), None);
            let node_id_unbonded_leftover = test
                .add_rewarded_legacy_mixnode(&test.make_addr("mix-owner-unbonded-leftover"), None);
            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![
                node_id_unbonded,
                node_id_unbonded_leftover,
                node_id_never_existed,
            ]);
            test.start_epoch_transition();

            let env = test.env();

            // note: we don't have to test for cases where `is_unbonding` is set to true on a mixnode
            // since before performing the nym-api should clear out the event queue

            // manually adjust delegation info as to indicate the rewarding information shouldnt get removed
            let mut rewarding_details = storage::NYMNODE_REWARDING
                .load(test.deps().storage, node_id_unbonded_leftover)
                .unwrap();
            rewarding_details.delegates = Decimal::raw(12345);
            rewarding_details.unique_delegations = 1;
            storage::NYMNODE_REWARDING
                .save(
                    test.deps_mut().storage,
                    node_id_unbonded_leftover,
                    &rewarding_details,
                )
                .unwrap();
            pending_events::unbond_mixnode(test.deps_mut(), &env, 123, node_id_unbonded).unwrap();

            pending_events::unbond_mixnode(test.deps_mut(), &env, 123, node_id_unbonded_leftover)
                .unwrap();

            let active_params = test.active_node_params(100.);

            for &node_id in &[
                node_id_unbonded,
                node_id_unbonded_leftover,
                node_id_never_existed,
            ] {
                let res = test.assert_rewarding(node_id, active_params);

                let reason = find_attribute(
                    Some(MixnetEventType::NodeRewarding.to_string()),
                    NO_REWARD_REASON_KEY,
                    &res,
                );
                assert_eq!(BOND_NOT_FOUND_VALUE, reason);
            }
        }

        #[test]
        fn can_only_be_performed_once_epoch_is_over() {
            let mut test = TestSetup::new();

            let node_id = test
                .add_rewarded_legacy_mixnode(&test.make_addr(test.make_addr("mix-owner")), None);

            // node is in the active set BUT the current epoch has just begun
            test.skip_to_next_epoch();
            test.force_change_mix_rewarded_set(vec![node_id]);

            let active_params = test.active_node_params(100.);
            let res = test.execute_rewarding(node_id, active_params);
            assert!(matches!(
                res,
                Err(MixnetContractError::EpochInProgress { .. })
            ));

            // epoch is over (sanity check)
            test.skip_to_current_epoch_end();
            test.start_epoch_transition();
            let res = test.execute_rewarding(node_id, active_params);
            assert!(res.is_ok());
        }

        #[test]
        fn can_only_be_performed_once_per_node_per_epoch() {
            let mut test = TestSetup::new();
            let node_id = test
                .add_rewarded_legacy_mixnode(&test.make_addr(test.make_addr("mix-owner")), None);

            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![node_id, 42]);
            test.start_epoch_transition();
            let active_params = test.active_node_params(100.);

            // first rewarding
            test.assert_rewarding(node_id, active_params);

            // second rewarding
            let res = test.execute_rewarding(node_id, active_params);
            assert!(matches!(
                res,
                Err(MixnetContractError::NodeAlreadyRewarded { node_id, .. }) if node_id == node_id
            ));

            // in the following epoch we're good again
            test.skip_to_next_epoch_end();
            test.start_epoch_transition();

            let res = test.execute_rewarding(node_id, active_params);
            assert!(res.is_ok());
        }

        #[test]
        fn requires_nonzero_performance_score() {
            let mut test = TestSetup::new();
            let node_id = test
                .add_rewarded_legacy_mixnode(&test.make_addr(test.make_addr("mix-owner")), None);

            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![node_id, 42]);
            test.start_epoch_transition();
            let zero_perf_params = test.active_node_params(0.);
            let active_params = test.active_node_params(100.);

            // first rewarding
            let res = test.assert_rewarding(node_id, zero_perf_params);
            let reason = res.attribute(MixnetEventType::NodeRewarding, NO_REWARD_REASON_KEY);
            assert_eq!(ZERO_PERFORMANCE_OR_WORK_VALUE, reason);

            // sanity check: it's still treated as rewarding, so we can't reward the node again
            // with different performance for the same epoch
            let res = test.execute_rewarding(node_id, zero_perf_params);
            assert!(matches!(
                res,
                Err(MixnetContractError::NodeAlreadyRewarded { node_id, .. }) if node_id == node_id
            ));

            // but in the next epoch, as always, we're good again
            test.skip_to_next_epoch_end();
            test.start_epoch_transition();

            let res = test.assert_rewarding(node_id, active_params);

            // rewards got distributed (in this test we don't care what they were exactly, but they must be non-zero)
            let operator = res.attribute(MixnetEventType::NodeRewarding, OPERATOR_REWARD_KEY);
            assert!(!operator.is_empty());
            assert_ne!("0", operator);
            let delegates = res.attribute(MixnetEventType::NodeRewarding, DELEGATES_REWARD_KEY);
            assert_eq!("0", delegates);
        }

        #[test]
        fn requires_nonzero_work_factor() {
            let mut test = TestSetup::new();
            let node_id = test
                .add_rewarded_legacy_mixnode(&test.make_addr(test.make_addr("mix-owner")), None);

            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![node_id, 42]);
            test.start_epoch_transition();

            let zero_work_params =
                NodeRewardingParameters::new(test_helpers::performance(100.), WorkFactor::zero());
            let active_params = test.active_node_params(100.);

            // first rewarding
            let res = test.assert_rewarding(node_id, zero_work_params);
            let reason = res.attribute(MixnetEventType::NodeRewarding, NO_REWARD_REASON_KEY);
            assert_eq!(ZERO_PERFORMANCE_OR_WORK_VALUE, reason);

            // sanity check: it's still treated as rewarding, so we can't reward the node again
            // with different performance for the same epoch
            let res = test.execute_rewarding(node_id, zero_work_params);
            assert!(matches!(
                res,
                Err(MixnetContractError::NodeAlreadyRewarded { node_id, .. }) if node_id == node_id
            ));

            // but in the next epoch, as always, we're good again
            test.skip_to_next_epoch_end();
            test.start_epoch_transition();

            let res = test.assert_rewarding(node_id, active_params);

            // rewards got distributed (in this test we don't care what they were exactly, but they must be non-zero)
            let operator = res.attribute(MixnetEventType::NodeRewarding, OPERATOR_REWARD_KEY);
            assert!(!operator.is_empty());
            assert_ne!("0", operator);
            let delegates = res.attribute(MixnetEventType::NodeRewarding, DELEGATES_REWARD_KEY);
            assert_eq!("0", delegates);
        }

        #[test]
        fn correctly_accounts_for_rewards_distributed() {
            let mut test = TestSetup::new();
            let node_id1 = test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner1"), None);
            let node_id2 = test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner2"), None);
            let node_id3 = test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner3"), None);

            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![node_id1, node_id2, node_id3]);
            test.start_epoch_transition();
            let params = test.active_node_params(98.0);

            test.add_immediate_delegation(
                &test.make_addr("delegator1"),
                Uint128::new(100_000_000),
                node_id2,
            );

            test.add_immediate_delegation(
                &test.make_addr("delegator1"),
                Uint128::new(100_000_000),
                node_id3,
            );
            test.add_immediate_delegation(
                &test.make_addr("delegator2"),
                Uint128::new(123_456_000),
                node_id3,
            );
            test.add_immediate_delegation(
                &test.make_addr("delegator3"),
                Uint128::new(9_100_000_000),
                node_id3,
            );

            let change = storage::PENDING_REWARD_POOL_CHANGE
                .load(test.deps().storage)
                .unwrap();
            assert!(change.removed.is_zero());
            assert!(change.added.is_zero());

            let mut total_operator = Decimal::zero();
            let mut total_delegates = Decimal::zero();

            for &node_id in &[node_id1, node_id2, node_id3] {
                let before = storage::NYMNODE_REWARDING
                    .load(test.deps().storage, node_id)
                    .unwrap();

                let res = test.assert_rewarding(node_id, params);
                let operator = res.decimal(MixnetEventType::NodeRewarding, OPERATOR_REWARD_KEY);
                let delegates = res.decimal(MixnetEventType::NodeRewarding, DELEGATES_REWARD_KEY);

                let after = storage::NYMNODE_REWARDING
                    .load(test.deps().storage, node_id)
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
            let global_rewarding_params = test.rewarding_params();
            let node_id1 =
                test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner1"), Some(operator1));
            let node_id2 =
                test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner2"), Some(operator2));
            let node_id3 =
                test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner3"), Some(operator3));

            test.skip_to_next_epoch_end();
            test.start_epoch_transition();
            test.force_change_mix_rewarded_set(vec![node_id1, node_id2, node_id3]);
            let performance = test_helpers::performance(98.0);

            test.add_immediate_delegation(
                &test.make_addr("delegator1"),
                Uint128::new(100_000_000),
                node_id2,
            );

            test.add_immediate_delegation(
                &test.make_addr("delegator1"),
                Uint128::new(100_000_000),
                node_id3,
            );
            test.add_immediate_delegation(
                &test.make_addr("delegator2"),
                Uint128::new(123_456_000),
                node_id3,
            );
            test.add_immediate_delegation(
                &test.make_addr("delegator3"),
                Uint128::new(9_100_000_000),
                node_id3,
            );

            // bypass proper epoch progression and force change the state
            test.set_epoch_in_progress_state();

            // repeat the rewarding the same set of delegates for few epochs
            for _ in 0..10 {
                test.start_epoch_transition();
                for &node_id in &[node_id1, node_id2, node_id3] {
                    let mut sim = test.instantiate_simulator(node_id);
                    let node_params = NodeRewardingParameters::new(
                        performance,
                        global_rewarding_params.active_node_work(),
                    );
                    let dist = test.reward_with_distribution(node_id, node_params);
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
            test.add_immediate_delegation(
                &test.make_addr("delegator1"),
                Uint128::new(50_000_000),
                node_id1,
            );
            test.add_immediate_delegation(
                &test.make_addr("delegator1"),
                Uint128::new(200_000_000),
                node_id2,
            );

            test.add_immediate_delegation(
                &test.make_addr("delegator5"),
                Uint128::new(123_000_000),
                node_id3,
            );
            test.add_immediate_delegation(
                &test.make_addr("delegator6"),
                Uint128::new(456_000_000),
                node_id3,
            );

            // bypass proper epoch progression and force change the state
            test.set_epoch_in_progress_state();

            let performance = test_helpers::performance(12.3);
            for _ in 0..10 {
                test.start_epoch_transition();
                for &node_id in &[node_id1, node_id2, node_id3] {
                    let mut sim = test.instantiate_simulator(node_id);
                    let node_params = NodeRewardingParameters::new(
                        performance,
                        global_rewarding_params.active_node_work(),
                    );
                    let dist = test.reward_with_distribution(node_id, node_params);
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
            let global_rewarding_params = test.rewarding_params();

            let node_id1 =
                test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner1"), Some(operator1));
            let node_id2 =
                test.add_rewarded_legacy_mixnode(&test.make_addr("mix-owner2"), Some(operator2));

            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![node_id1, node_id2]);
            let performance = test_helpers::performance(98.0);

            test.add_immediate_delegation(
                &test.make_addr("delegator1"),
                Uint128::new(100_000_000),
                node_id1,
            );
            test.add_immediate_delegation(
                &test.make_addr("delegator1"),
                Uint128::new(100_000_000),
                node_id2,
            );

            test.add_immediate_delegation(
                &test.make_addr("delegator2"),
                Uint128::new(123_456_000),
                node_id1,
            );

            let del11 = test.delegation(node_id1, &test.make_addr("delegator1"), &None);
            let del12 = test.delegation(node_id1, &test.make_addr("delegator2"), &None);
            let del21 = test.delegation(node_id2, &test.make_addr("delegator1"), &None);

            for _ in 0..10 {
                test.start_epoch_transition();

                // we know from the previous tests that actual rewarding distribution matches the simulator
                let mut sim1 = test.instantiate_simulator(node_id1);
                let mut sim2 = test.instantiate_simulator(node_id2);

                let node_params = NodeRewardingParameters::new(
                    performance,
                    global_rewarding_params.active_node_work(),
                );

                let dist1 = sim1.simulate_epoch_single_node(node_params).unwrap();
                let dist2 = sim2.simulate_epoch_single_node(node_params).unwrap();

                let actual_prior1 = test.mix_rewarding(node_id1);
                let actual_prior2 = test.mix_rewarding(node_id2);

                let res1 = test.assert_rewarding(node_id1, node_params);

                let prior_delegates1 =
                    res1.decimal(MixnetEventType::NodeRewarding, PRIOR_DELEGATES_KEY);
                let delegates_reward1 =
                    res1.decimal(MixnetEventType::NodeRewarding, DELEGATES_REWARD_KEY);
                let prior_unit_reward =
                    res1.decimal(MixnetEventType::NodeRewarding, PRIOR_UNIT_REWARD_KEY);

                assert_eq!(prior_delegates1, actual_prior1.delegates);
                assert_eq!(delegates_reward1, dist1.delegates);
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

                let res2 = test.assert_rewarding(node_id2, node_params);

                let prior_delegates2 =
                    res2.decimal(MixnetEventType::NodeRewarding, PRIOR_DELEGATES_KEY);
                let delegates_reward2 =
                    res2.decimal(MixnetEventType::NodeRewarding, DELEGATES_REWARD_KEY);
                let prior_unit_reward =
                    res2.decimal(MixnetEventType::NodeRewarding, PRIOR_UNIT_REWARD_KEY);

                assert_eq!(prior_delegates2, actual_prior2.delegates);
                assert_eq!(delegates_reward2, dist2.delegates);
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
            test.add_immediate_delegation(
                &test.make_addr("delegator3"),
                Uint128::new(15_850_000_000),
                node_id1,
            );
            test.add_immediate_delegation(
                &test.make_addr("delegator3"),
                Uint128::new(15_850_000_000),
                node_id2,
            );

            let del13 = test.delegation(node_id1, &test.make_addr("delegator3"), &None);
            let del23 = test.delegation(node_id2, &test.make_addr("delegator3"), &None);

            for _ in 0..10 {
                test.start_epoch_transition();

                // we know from the previous tests that actual rewarding distribution matches the simulator
                let mut sim1 = test.instantiate_simulator(node_id1);
                let mut sim2 = test.instantiate_simulator(node_id2);

                let node_params = NodeRewardingParameters::new(
                    performance,
                    global_rewarding_params.active_node_work(),
                );

                let dist1 = sim1.simulate_epoch_single_node(node_params).unwrap();
                let dist2 = sim2.simulate_epoch_single_node(node_params).unwrap();

                let actual_prior1 = test.mix_rewarding(node_id1);
                let actual_prior2 = test.mix_rewarding(node_id2);

                let res1 = test.assert_rewarding(node_id1, node_params);

                let prior_delegates1 =
                    res1.decimal(MixnetEventType::NodeRewarding, PRIOR_DELEGATES_KEY);
                let delegates_reward1 =
                    res1.decimal(MixnetEventType::NodeRewarding, DELEGATES_REWARD_KEY);
                let prior_unit_reward =
                    res1.decimal(MixnetEventType::NodeRewarding, PRIOR_UNIT_REWARD_KEY);

                assert_eq!(prior_delegates1, actual_prior1.delegates);
                assert_eq!(delegates_reward1, dist1.delegates);
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

                let res2 = test.assert_rewarding(node_id2, node_params);

                let prior_delegates2 =
                    res2.decimal(MixnetEventType::NodeRewarding, PRIOR_DELEGATES_KEY);
                let delegates_reward2 =
                    res2.decimal(MixnetEventType::NodeRewarding, DELEGATES_REWARD_KEY);
                let prior_unit_reward =
                    res2.decimal(MixnetEventType::NodeRewarding, PRIOR_UNIT_REWARD_KEY);

                assert_eq!(prior_delegates2, actual_prior2.delegates);
                assert_eq!(delegates_reward2, dist2.delegates);
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
    mod legacy_gateway_rewarding {
        use super::*;
        use crate::support::tests::test_helpers::FindAttribute;
        use mixnet_contract_common::events::{BOND_NOT_FOUND_VALUE, NO_REWARD_REASON_KEY};
        use mixnet_contract_common::nym_node::Role;
        use mixnet_contract_common::RoleAssignment;

        #[test]
        fn regardless_of_performance_or_work_they_get_nothing() {
            let mut test = TestSetup::new();
            let (_, node_id) = test.add_legacy_gateway(&test.make_addr("owner"), None);

            test.skip_to_next_epoch_end();
            test.force_assign_rewarded_set(vec![RoleAssignment::new(
                Role::EntryGateway,
                vec![node_id],
            )]);
            test.start_epoch_transition();

            let rewarding_params = test.active_node_params(100.);
            let res = test.assert_rewarding(node_id, rewarding_params);

            let reward_attr = res.any_attribute(NO_REWARD_REASON_KEY);
            assert_eq!(reward_attr, BOND_NOT_FOUND_VALUE);

            // make sure the epoch actually progressed (i.e. unrewarded gateway hasn't stalled it)
            let current = test.current_epoch_state();
            assert_eq!(current, EpochState::ReconcilingEvents)
        }
    }

    // rewarding for entry gateway, exit gateway and standby nym-nodes
    #[cfg(test)]
    mod non_legacy_rewarding {
        use super::*;
        use crate::interval::pending_events;
        use crate::support::tests::test_helpers::FindAttribute;
        use cosmwasm_std::{Decimal, Uint128};
        use mixnet_contract_common::events::{
            BOND_NOT_FOUND_VALUE, NO_REWARD_REASON_KEY, OPERATOR_REWARD_KEY,
            ZERO_PERFORMANCE_OR_WORK_VALUE,
        };
        use mixnet_contract_common::nym_node::Role;
        use mixnet_contract_common::reward_params::WorkFactor;
        use mixnet_contract_common::RoleAssignment;
        use std::collections::HashMap;
        use std::ops::{Deref, DerefMut};

        struct RewardingSetup {
            standby_node: NodeId,
            entry_node: NodeId,
            exit_node: NodeId,
            mixing_node: NodeId,

            inner: TestSetup,
        }

        impl RewardingSetup {
            pub fn new_rewarding_setup() -> Self {
                let mut inner = TestSetup::new();
                let mixing_node = inner.add_dummy_nymnode(&inner.make_addr("mixing-owner"), None);
                let entry_node = inner.add_dummy_nymnode(&inner.make_addr("entry-owner"), None);
                let exit_node = inner.add_dummy_nymnode(&inner.make_addr("exit-owner"), None);
                let standby_node = inner.add_dummy_nymnode(&inner.make_addr("standby-owner"), None);

                RewardingSetup {
                    standby_node,
                    entry_node,
                    exit_node,
                    mixing_node,
                    inner,
                }
            }

            pub fn nodes(&self) -> Vec<NodeId> {
                vec![
                    self.mixing_node,
                    self.entry_node,
                    self.exit_node,
                    self.standby_node,
                ]
            }

            pub fn reset_rewarded_set(&mut self) {
                self.inner.force_assign_rewarded_set(vec![
                    RoleAssignment {
                        role: Role::Layer1,
                        nodes: vec![self.mixing_node],
                    },
                    RoleAssignment {
                        role: Role::EntryGateway,
                        nodes: vec![self.entry_node],
                    },
                    RoleAssignment {
                        role: Role::ExitGateway,
                        nodes: vec![self.exit_node],
                    },
                    RoleAssignment {
                        role: Role::Standby,
                        nodes: vec![self.standby_node],
                    },
                ]);
            }

            pub fn local_node_role(&self, node_id: NodeId) -> Role {
                match node_id {
                    n if n == self.mixing_node => Role::Layer1,
                    n if n == self.entry_node => Role::EntryGateway,
                    n if n == self.exit_node => Role::ExitGateway,
                    n if n == self.standby_node => Role::Standby,
                    _ => unreachable!(),
                }
            }

            pub fn add_to_rewarded_set(&mut self, node_id: NodeId) {
                let role = self.local_node_role(node_id);
                self.inner.force_assign_rewarded_set(vec![RoleAssignment {
                    role,
                    nodes: vec![node_id],
                }])
            }

            pub fn reward_all(
                &mut self,
                performance: f32,
            ) -> HashMap<NodeId, Result<Response, MixnetContractError>> {
                let mut results = HashMap::new();

                self.skip_to_next_epoch_end();
                self.reset_rewarded_set();
                self.start_epoch_transition();

                let active_params = self.active_node_params(performance);
                let standby_params = self.standby_node_params(performance);

                let mixing_node = self.mixing_node;
                let entry_node = self.entry_node;
                let exit_node = self.exit_node;
                let standby_node = self.standby_node;

                results.insert(
                    mixing_node,
                    self.execute_rewarding(mixing_node, active_params),
                );
                results.insert(
                    entry_node,
                    self.execute_rewarding(entry_node, active_params),
                );
                results.insert(exit_node, self.execute_rewarding(exit_node, active_params));
                results.insert(
                    standby_node,
                    self.execute_rewarding(standby_node, standby_params),
                );

                results
            }
        }

        impl Deref for RewardingSetup {
            type Target = TestSetup;
            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }

        impl DerefMut for RewardingSetup {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.inner
            }
        }

        #[test]
        fn when_target_node_has_zero_performance() {
            let mut test = RewardingSetup::new_rewarding_setup();
            let results = test.reward_all(0.);
            for res in results.into_values() {
                let reward_attr = res.unwrap().any_attribute(NO_REWARD_REASON_KEY);
                assert_eq!(reward_attr, ZERO_PERFORMANCE_OR_WORK_VALUE);
            }

            let current = test.current_epoch_state();
            assert_eq!(current, EpochState::ReconcilingEvents)
        }

        #[test]
        fn when_target_node_has_zero_work_factor() {
            let mut test = RewardingSetup::new_rewarding_setup();

            test.skip_to_next_epoch_end();
            test.reset_rewarded_set();
            test.start_epoch_transition();

            let params =
                NodeRewardingParameters::new(test_helpers::performance(100.), WorkFactor::zero());

            for node in test.nodes() {
                let res = test.assert_rewarding(node, params);
                let reward_attr = res.any_attribute(NO_REWARD_REASON_KEY);
                assert_eq!(reward_attr, ZERO_PERFORMANCE_OR_WORK_VALUE);
            }

            let current = test.current_epoch_state();
            assert_eq!(current, EpochState::ReconcilingEvents)
        }

        #[test]
        fn when_theres_only_one_node_to_reward() {
            let test_lookup = RewardingSetup::new_rewarding_setup();

            for node in test_lookup.nodes() {
                let mut actual_setup = RewardingSetup::new_rewarding_setup();
                actual_setup.add_to_rewarded_set(node);
                let mut res = actual_setup.reward_all(100.);

                // get the response for this particular node
                let res = res.remove(&node).unwrap().unwrap();
                let reward: Decimal = res.any_parsed_attribute(OPERATOR_REWARD_KEY);
                assert!(!reward.is_zero());

                let current = actual_setup.current_epoch_state();
                assert_eq!(current, EpochState::ReconcilingEvents)
            }
        }

        #[test]
        fn when_theres_multiple_nodes_to_reward() {
            let mut test = RewardingSetup::new_rewarding_setup();
            let results = test.reward_all(100.);
            for res in results.into_values() {
                let reward: Decimal = res.unwrap().any_parsed_attribute(OPERATOR_REWARD_KEY);
                assert!(!reward.is_zero());
            }

            let current = test.current_epoch_state();
            assert_eq!(current, EpochState::ReconcilingEvents)
        }

        #[test]
        fn cant_be_performed_for_unbonded_nodes() {
            let test_lookup = RewardingSetup::new_rewarding_setup();

            for node in test_lookup.nodes() {
                let mut actual_setup = RewardingSetup::new_rewarding_setup();
                actual_setup.add_to_rewarded_set(node);

                let env = actual_setup.env();

                let delegator = actual_setup.make_addr("delegator");
                // add some delegations to indicate the rewarding information shouldn't get removed
                actual_setup.add_immediate_delegation(&delegator, Uint128::new(12345678), node);
                pending_events::unbond_nym_node(actual_setup.deps_mut(), &env, 123, node).unwrap();

                let mut res = actual_setup.reward_all(100.);

                // get the response for this particular node
                let res = res.remove(&node).unwrap().unwrap();
                let reward_attr = res.any_attribute(NO_REWARD_REASON_KEY);
                assert_eq!(reward_attr, BOND_NOT_FOUND_VALUE);

                let current = actual_setup.current_epoch_state();
                assert_eq!(current, EpochState::ReconcilingEvents)
            }
        }

        #[test]
        fn can_only_be_performed_once_per_node_per_epoch() {
            let test_lookup = RewardingSetup::new_rewarding_setup();

            let params = test_lookup.active_node_params(100.0);
            for node in test_lookup.nodes() {
                let mut actual_setup = RewardingSetup::new_rewarding_setup();

                actual_setup.skip_to_next_epoch_end();

                let addr = &actual_setup.make_addr("foomp");
                let extra = actual_setup.add_dummy_nymnode(addr, None);

                // add extra node to the rewarded set so rewarding wouldn't immediately go into event reconciliation
                let role = actual_setup.local_node_role(node);
                actual_setup
                    .inner
                    .force_assign_rewarded_set(vec![RoleAssignment {
                        role,
                        nodes: vec![node, extra],
                    }]);

                actual_setup.start_epoch_transition();

                // first rewarding
                actual_setup.assert_rewarding(node, params);

                // second rewarding
                let res = actual_setup.execute_rewarding(node, params).unwrap_err();
                assert_eq!(
                    res,
                    MixnetContractError::NodeAlreadyRewarded {
                        node_id: node,
                        absolute_epoch_id: 1,
                    }
                );
            }
        }
    }

    #[cfg(test)]
    mod withdrawing_delegator_reward {
        use crate::interval::pending_events;
        use crate::support::tests::test_helpers::{assert_eq_with_leeway, TestSetup};
        use cosmwasm_std::testing::message_info;
        use cosmwasm_std::{BankMsg, CosmosMsg, Decimal, Uint128};
        use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;

        use super::*;

        #[test]
        fn can_only_be_done_if_delegation_exists() {
            let mut test = TestSetup::new();
            // add relatively huge stake so that the reward would be high enough to offset operating costs
            let node_id1 = test.add_rewarded_legacy_mixnode(
                &test.make_addr("mix-owner1"),
                Some(Uint128::new(1_000_000_000_000)),
            );
            let node_id2 = test.add_rewarded_legacy_mixnode(
                &test.make_addr("mix-owner2"),
                Some(Uint128::new(1_000_000_000_000)),
            );
            let active_params = test.active_node_params(100.);

            let delegator1 = test.make_addr("delegator1");
            let delegator2 = test.make_addr("delegator2");

            let sender1 = message_info(&delegator1, &[]);
            let sender2 = message_info(&delegator2, &[]);

            // note that there's no delegation from delegator1 towards mix1
            test.add_immediate_delegation(&delegator2, 100_000_000u128, node_id1);

            test.add_immediate_delegation(&delegator1, 100_000_000u128, node_id2);
            test.add_immediate_delegation(&delegator2, 100_000_000u128, node_id2);

            // perform some rewarding so that we'd have non-zero rewards
            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![node_id1, node_id2]);
            test.start_epoch_transition();
            test.reward_with_distribution(node_id1, active_params);
            test.reward_with_distribution(node_id2, active_params);

            let res = try_withdraw_delegator_reward(test.deps_mut(), sender1.clone(), node_id1);
            assert_eq!(
                res,
                Err(MixnetContractError::NodeDelegationNotFound {
                    node_id: node_id1,
                    address: delegator1.to_string(),
                    proxy: None,
                })
            );

            // sanity check for other ones
            let res = try_withdraw_delegator_reward(test.deps_mut(), sender1, node_id2);
            assert!(res.is_ok());

            let res = try_withdraw_delegator_reward(test.deps_mut(), sender2.clone(), node_id1);
            assert!(res.is_ok());

            let res = try_withdraw_delegator_reward(test.deps_mut(), sender2, node_id2);
            assert!(res.is_ok());
        }

        #[test]
        fn tokens_are_only_sent_if_reward_is_nonzero() {
            let mut test = TestSetup::new();
            // add relatively huge stake so that the reward would be high enough to offset operating costs
            let node_id1 = test.add_rewarded_legacy_mixnode(
                &test.make_addr("mix-owner1"),
                Some(Uint128::new(1_000_000_000_000)),
            );
            let node_id2 = test.add_rewarded_legacy_mixnode(
                &test.make_addr("mix-owner2"),
                Some(Uint128::new(1_000_000_000_000)),
            );
            let active_params = test.active_node_params(100.);

            // very low stake so operating cost would be higher than total reward
            let low_stake_id = test.add_rewarded_legacy_mixnode(
                &test.make_addr("mix-owner3"),
                Some(Uint128::new(100_000_000)),
            );

            let delegator = test.make_addr("delegator");
            let sender = message_info(&delegator, &[]);

            test.add_immediate_delegation(&delegator, 100_000_000u128, node_id1);
            test.add_immediate_delegation(&delegator, 100_000_000u128, node_id2);
            test.add_immediate_delegation(&delegator, 1_000u128, low_stake_id);

            // reward mix1, but don't reward mix2
            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![node_id1, low_stake_id]);
            test.start_epoch_transition();
            test.reward_with_distribution(node_id1, active_params);
            test.reward_with_distribution(low_stake_id, active_params);

            let res1 =
                try_withdraw_delegator_reward(test.deps_mut(), sender.clone(), node_id1).unwrap();
            assert!(matches!(
                &res1.messages[0].msg,
                CosmosMsg::Bank(BankMsg::Send { to_address, amount }) if to_address == delegator.as_str() && !amount[0].amount.is_zero()
            ),);

            let res2 =
                try_withdraw_delegator_reward(test.deps_mut(), sender.clone(), node_id2).unwrap();
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
            let node_id_unbonding = test.add_rewarded_legacy_mixnode(
                &test.make_addr("mix-owner1"),
                Some(Uint128::new(1_000_000_000_000)),
            );
            let node_id_unbonded_leftover = test.add_rewarded_legacy_mixnode(
                &test.make_addr("mix-owner2"),
                Some(Uint128::new(1_000_000_000_000)),
            );

            let delegator = test.make_addr("delegator");
            let sender = message_info(&delegator, &[]);

            test.add_immediate_delegation(&delegator, 100_000_000u128, node_id_unbonding);
            test.add_immediate_delegation(&delegator, 100_000_000u128, node_id_unbonded_leftover);

            let active_params = test.active_node_params(100.);
            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![node_id_unbonding, node_id_unbonded_leftover]);

            // go through few rewarding cycles before unbonding nodes (partially or fully)
            for _ in 0..10 {
                test.start_epoch_transition();

                test.reward_with_distribution(node_id_unbonding, active_params);
                test.reward_with_distribution(node_id_unbonded_leftover, active_params);

                test.skip_to_next_epoch_end();
                // bypass proper epoch progression and force change the state
                test.set_epoch_in_progress_state();
            }

            // start unbonding the first node and fully unbond the other
            let mut bond = mixnodes_storage::mixnode_bonds()
                .load(test.deps().storage, node_id_unbonding)
                .unwrap();
            bond.is_unbonding = true;
            mixnodes_storage::mixnode_bonds()
                .save(test.deps_mut().storage, node_id_unbonding, &bond)
                .unwrap();

            let env = test.env();
            pending_events::unbond_mixnode(test.deps_mut(), &env, 123, node_id_unbonded_leftover)
                .unwrap();

            let res =
                try_withdraw_delegator_reward(test.deps_mut(), sender.clone(), node_id_unbonding);
            assert_eq!(
                res,
                Err(MixnetContractError::MixnodeIsUnbonding {
                    mix_id: node_id_unbonding
                })
            );

            let res =
                try_withdraw_delegator_reward(test.deps_mut(), sender, node_id_unbonded_leftover);
            assert_eq!(
                res,
                Err(MixnetContractError::NymNodeBondNotFound {
                    node_id: node_id_unbonded_leftover
                })
            );
        }

        #[test]
        fn correctly_determines_earned_share_and_resets_reward_ratio() {
            let mut test = TestSetup::new();
            let node_id_single = test.add_rewarded_legacy_mixnode(
                &test.make_addr("mix-owner1"),
                Some(Uint128::new(1_000_000_000_000)),
            );
            let node_id_quad = test.add_rewarded_legacy_mixnode(
                &test.make_addr("mix-owner2"),
                Some(Uint128::new(1_000_000_000_000)),
            );

            let delegator1 = test.make_addr("delegator1");
            let delegator2 = test.make_addr("delegator2");
            let delegator3 = test.make_addr("delegator3");
            let delegator4 = test.make_addr("delegator4");
            let sender1 = message_info(&delegator1, &[]);
            let sender2 = message_info(&delegator2, &[]);
            let sender3 = message_info(&delegator3, &[]);
            let sender4 = message_info(&delegator4, &[]);

            let amount_single = 100_000_000u128;

            let amount_quad1 = 50_000_000u128;
            let amount_quad2 = 200_000_000u128;
            let amount_quad3 = 250_000_000u128;
            let amount_quad4 = 500_000_000u128;

            test.add_immediate_delegation(&delegator1, amount_single, node_id_single);

            test.add_immediate_delegation(&delegator1, amount_quad1, node_id_quad);
            test.add_immediate_delegation(&delegator2, amount_quad2, node_id_quad);
            test.add_immediate_delegation(&delegator3, amount_quad3, node_id_quad);
            test.add_immediate_delegation(&delegator4, amount_quad4, node_id_quad);

            let active_params = test.active_node_params(100.);
            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![node_id_single, node_id_quad]);

            // accumulate some rewards
            let mut accumulated_single = Decimal::zero();
            let mut accumulated_quad = Decimal::zero();
            for _ in 0..10 {
                test.start_epoch_transition();
                let dist = test.reward_with_distribution(node_id_single, active_params);
                // sanity check to make sure test is actually doing what it's supposed to be doing
                assert!(!dist.delegates.is_zero());

                accumulated_single += dist.delegates;
                let dist = test.reward_with_distribution(node_id_quad, active_params);
                accumulated_quad += dist.delegates;

                test.skip_to_next_epoch_end();
                // bypass proper epoch progression and force change the state
                test.set_epoch_in_progress_state();
            }

            let before = test.read_delegation(node_id_single, &delegator1, None);
            assert_eq!(before.cumulative_reward_ratio, Decimal::zero());
            let res1 =
                try_withdraw_delegator_reward(test.deps_mut(), sender1.clone(), node_id_single)
                    .unwrap();
            let (_, reward) = test_helpers::get_bank_send_msg(&res1).unwrap();
            assert_eq!(truncate_reward_amount(accumulated_single), reward[0].amount);
            let after = test.read_delegation(node_id_single, &delegator1, None);
            assert_ne!(after.cumulative_reward_ratio, Decimal::zero());
            assert_eq!(
                after.cumulative_reward_ratio,
                test.mix_rewarding(node_id_single).total_unit_reward
            );

            // withdraw first two rewards. note that due to scaling we expect second reward to be 4x the first one
            let before1 = test.read_delegation(node_id_quad, &delegator1, None);
            let before2 = test.read_delegation(node_id_quad, &delegator2, None);
            assert_eq!(before1.cumulative_reward_ratio, Decimal::zero());
            assert_eq!(before2.cumulative_reward_ratio, Decimal::zero());
            let res1 =
                try_withdraw_delegator_reward(test.deps_mut(), sender1.clone(), node_id_quad)
                    .unwrap();
            let (_, reward1) = test_helpers::get_bank_send_msg(&res1).unwrap();
            let res2 =
                try_withdraw_delegator_reward(test.deps_mut(), sender2.clone(), node_id_quad)
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

            let after1 = test.read_delegation(node_id_quad, &delegator1, None);
            assert_ne!(after1.cumulative_reward_ratio, Decimal::zero());
            assert_eq!(
                after1.cumulative_reward_ratio,
                test.mix_rewarding(node_id_quad).total_unit_reward
            );
            let after2 = test.read_delegation(node_id_quad, &delegator2, None);
            assert_ne!(after2.cumulative_reward_ratio, Decimal::zero());
            assert_eq!(
                after2.cumulative_reward_ratio,
                test.mix_rewarding(node_id_quad).total_unit_reward
            );

            // accumulate some more
            for _ in 0..10 {
                test.start_epoch_transition();

                let dist = test.reward_with_distribution(node_id_quad, active_params);
                accumulated_quad += dist.delegates;
                test.skip_to_next_epoch_end();
                // bypass proper epoch progression and force change the state
                test.set_epoch_in_progress_state();
            }

            let before1_new = test.read_delegation(node_id_quad, &delegator1, None);
            let before2_new = test.read_delegation(node_id_quad, &delegator2, None);
            let before3 = test.read_delegation(node_id_quad, &delegator3, None);
            let before4 = test.read_delegation(node_id_quad, &delegator4, None);

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
                try_withdraw_delegator_reward(test.deps_mut(), sender1, node_id_quad).unwrap();
            let (_, reward1_new) = test_helpers::get_bank_send_msg(&res1).unwrap();
            let res2 =
                try_withdraw_delegator_reward(test.deps_mut(), sender2, node_id_quad).unwrap();
            let (_, reward2_new) = test_helpers::get_bank_send_msg(&res2).unwrap();

            // the ratio between first and second delegator is still the same
            assert_eq_with_leeway(
                reward1_new[0].amount * Uint128::new(4),
                reward2_new[0].amount,
                Uint128::new(4),
            );

            let res3 =
                try_withdraw_delegator_reward(test.deps_mut(), sender3, node_id_quad).unwrap();
            let (_, reward3) = test_helpers::get_bank_send_msg(&res3).unwrap();
            let res4 =
                try_withdraw_delegator_reward(test.deps_mut(), sender4, node_id_quad).unwrap();
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
        use crate::compat::transactions::try_withdraw_operator_reward;
        use crate::interval::pending_events;
        use crate::support::tests::test_helpers::TestSetup;
        use cosmwasm_std::{Addr, BankMsg, CosmosMsg, Uint128};

        #[test]
        fn can_only_be_done_if_bond_exists() {
            let mut test = TestSetup::new();

            let owner = &test.make_addr(test.make_addr("mix-owner"));
            let node_id =
                test.add_rewarded_legacy_mixnode(owner, Some(Uint128::new(1_000_000_000_000)));
            let sender = message_info(&test.make_addr("random-guy"), &[]);
            let active_params = test.active_node_params(100.);

            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![node_id]);
            test.start_epoch_transition();
            test.reward_with_distribution(node_id, active_params);

            let res = try_withdraw_operator_reward(test.deps_mut(), sender.clone());
            assert_eq!(
                res,
                Err(MixnetContractError::NoAssociatedNodeBond {
                    owner: sender.sender
                })
            )
        }

        #[test]
        fn tokens_are_only_sent_if_reward_is_nonzero() {
            let mut test = TestSetup::new();

            let owner1 = &test.make_addr("mix-owner1");
            let owner2 = &test.make_addr("mix-owner2");
            let node_id1 =
                test.add_rewarded_legacy_mixnode(owner1, Some(Uint128::new(1_000_000_000_000)));
            test.add_rewarded_legacy_mixnode(owner2, Some(Uint128::new(1_000_000_000_000)));
            let active_params = test.active_node_params(100.);

            let sender1 = message_info(owner1, &[]);
            let sender2 = message_info(owner2, &[]);

            // reward mix1, but don't reward mix2
            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![node_id1]);
            test.start_epoch_transition();
            test.reward_with_distribution(node_id1, active_params);

            let res1 = try_withdraw_operator_reward(test.deps_mut(), sender1).unwrap();
            assert!(matches!(
                &res1.messages[0].msg,
                CosmosMsg::Bank(BankMsg::Send { to_address, amount }) if to_address == owner1.as_str() && !amount[0].amount.is_zero()
            ),);

            let res2 = try_withdraw_operator_reward(test.deps_mut(), sender2).unwrap();
            assert!(res2.messages.is_empty());
        }

        #[test]
        fn can_only_be_done_for_fully_bonded_nodes() {
            // note: if node has unbonded or is in the process of unbonding, the expected
            // way of getting back the rewards is finish the undelegation
            let mut test = TestSetup::new();
            let owner1 = &test.make_addr("mix-owner1");
            let owner2 = &test.make_addr("mix-owner2");
            let sender1 = message_info(owner1, &[]);
            let sender2 = message_info(owner2, &[]);
            let node_id_unbonding =
                test.add_rewarded_legacy_mixnode(owner1, Some(Uint128::new(1_000_000_000_000)));
            let node_id_unbonded_leftover =
                test.add_rewarded_legacy_mixnode(owner2, Some(Uint128::new(1_000_000_000_000)));

            // add some delegation to the second node so that it wouldn't be cleared upon unbonding
            test.add_immediate_delegation(
                &test.make_addr("delegator"),
                100_000_000u128,
                node_id_unbonded_leftover,
            );

            let active_params = test.active_node_params(100.);
            test.skip_to_next_epoch_end();
            test.force_change_mix_rewarded_set(vec![node_id_unbonding, node_id_unbonded_leftover]);

            // go through few rewarding cycles before unbonding nodes (partially or fully)
            for _ in 0..10 {
                test.start_epoch_transition();
                test.reward_with_distribution(node_id_unbonding, active_params);
                test.reward_with_distribution(node_id_unbonded_leftover, active_params);

                test.skip_to_next_epoch_end();
                // bypass proper epoch progression and force change the state
                test.set_epoch_in_progress_state();
            }

            // start unbonding the first node and fully unbond the other
            let mut bond = mixnodes_storage::mixnode_bonds()
                .load(test.deps().storage, node_id_unbonding)
                .unwrap();
            bond.is_unbonding = true;
            mixnodes_storage::mixnode_bonds()
                .save(test.deps_mut().storage, node_id_unbonding, &bond)
                .unwrap();

            let env = test.env();
            pending_events::unbond_mixnode(test.deps_mut(), &env, 123, node_id_unbonded_leftover)
                .unwrap();

            let res = try_withdraw_operator_reward(test.deps_mut(), sender1);
            assert_eq!(
                res,
                Err(MixnetContractError::NodeIsUnbonding {
                    node_id: node_id_unbonding
                })
            );

            let res = try_withdraw_operator_reward(test.deps_mut(), sender2);
            assert_eq!(
                res,
                Err(MixnetContractError::NoAssociatedNodeBond {
                    owner: Addr::unchecked(owner2)
                })
            );
        }
    }

    #[cfg(test)]
    mod updating_active_set {
        use cw_controllers::AdminError::NotAdmin;
        use mixnet_contract_common::nym_node::Role;
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
                EpochState::RoleAssignment {
                    next: Role::first(),
                },
            ];

            for bad_state in bad_states {
                let mut test = TestSetup::new();
                let owner = test.owner();
                let env = test.env();

                let mut status = EpochStatus::new(test.rewarding_validator().sender);
                status.state = bad_state;

                interval_storage::save_current_epoch_status(test.deps_mut().storage, &status)
                    .unwrap();

                let res = try_update_active_set_distribution(
                    test.deps_mut(),
                    env.clone(),
                    owner.clone(),
                    active_set_update_fixture(),
                    false,
                );
                assert!(matches!(
                    res,
                    Err(MixnetContractError::EpochAdvancementInProgress { .. })
                ));

                let res_forced = try_update_active_set_distribution(
                    test.deps_mut(),
                    env.clone(),
                    owner,
                    active_set_update_fixture(),
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
            let random = message_info(&test.make_addr("random-guy"), &[]);

            let env = test.env();
            let res = try_update_active_set_distribution(
                test.deps_mut(),
                env.clone(),
                rewarding_validator,
                active_set_update_fixture(),
                false,
            );
            assert_eq!(res, Err(MixnetContractError::Admin(NotAdmin {})));

            let res = try_update_active_set_distribution(
                test.deps_mut(),
                env.clone(),
                random,
                active_set_update_fixture(),
                false,
            );
            assert_eq!(res, Err(MixnetContractError::Admin(NotAdmin {})));

            let res = try_update_active_set_distribution(
                test.deps_mut(),
                env,
                owner,
                active_set_update_fixture(),
                false,
            );
            assert!(res.is_ok())
        }

        #[test]
        fn new_size_cant_be_bigger_than_rewarded_set() {
            let mut test = TestSetup::new();
            let owner = test.owner();

            let rewarded_set_size = storage::REWARDING_PARAMS
                .load(test.deps().storage)
                .unwrap()
                .rewarded_set_size();

            let env = test.env();
            let res = try_update_active_set_distribution(
                test.deps_mut(),
                env,
                owner.clone(),
                ActiveSetUpdate {
                    entry_gateways: rewarded_set_size,
                    exit_gateways: rewarded_set_size,
                    mixnodes: rewarded_set_size * 3,
                },
                false,
            );
            assert_eq!(res, Err(MixnetContractError::InvalidActiveSetSize));

            // if its equal, its fine
            // (make sure we start with the fresh state)
            let mut test = TestSetup::new();
            let env = test.env();
            let res = try_update_active_set_distribution(
                test.deps_mut(),
                env,
                owner.clone(),
                ActiveSetUpdate {
                    entry_gateways: rewarded_set_size / 3,
                    exit_gateways: rewarded_set_size / 3,
                    mixnodes: rewarded_set_size / 3,
                },
                false,
            );
            assert!(res.is_ok());

            // as well as if its any value lower than that
            let mut test = TestSetup::new();
            let env = test.env();
            let res = try_update_active_set_distribution(
                test.deps_mut(),
                env,
                owner,
                ActiveSetUpdate {
                    entry_gateways: 1,
                    exit_gateways: 1,
                    mixnodes: 3,
                },
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
                .active_set_size();
            test.skip_to_current_interval_end();
            let env = test.env();

            let update = active_set_update_fixture();
            let expected = update.active_set_size();
            let res = try_update_active_set_distribution(
                test.deps_mut(),
                env,
                owner.clone(),
                update,
                false,
            );
            assert!(res.is_ok());
            let new = storage::REWARDING_PARAMS
                .load(test.deps().storage)
                .unwrap()
                .active_set_size();
            assert_ne!(old, new);
            assert_eq!(new, expected);

            // sanity check for "normal" case
            let mut test = TestSetup::new();
            let env = test.env();
            let res =
                try_update_active_set_distribution(test.deps_mut(), env, owner, update, false);
            assert!(res.is_ok());
            let new = storage::REWARDING_PARAMS
                .load(test.deps().storage)
                .unwrap()
                .active_set_size();
            assert_eq!(old, new);
        }

        #[test]
        fn if_update_is_forced_it_happens_immediately() {
            let mut test = TestSetup::new();
            let owner = test.owner();

            let old = storage::REWARDING_PARAMS
                .load(test.deps().storage)
                .unwrap()
                .active_set_size();
            let env = test.env();

            let update = active_set_update_fixture();
            let expected = update.active_set_size();
            let res = try_update_active_set_distribution(test.deps_mut(), env, owner, update, true);
            assert!(res.is_ok());
            let new = storage::REWARDING_PARAMS
                .load(test.deps().storage)
                .unwrap()
                .active_set_size();
            assert_ne!(old, new);
            assert_eq!(new, expected);
        }

        #[test]
        fn without_forcing_it_change_happens_upon_clearing_epoch_events() {
            let mut test = TestSetup::new();
            let owner = test.owner();

            let old = storage::REWARDING_PARAMS
                .load(test.deps().storage)
                .unwrap()
                .active_set_size();
            let env = test.env();

            let issued_update = active_set_update_fixture();
            let expected_updated = issued_update.active_set_size();
            let res = try_update_active_set_distribution(
                test.deps_mut(),
                env,
                owner,
                issued_update,
                false,
            );
            assert!(res.is_ok());
            let new = storage::REWARDING_PARAMS
                .load(test.deps().storage)
                .unwrap()
                .active_set_size();
            assert_eq!(old, new);

            // make sure it's actually saved to pending events
            let events = test.pending_epoch_events();
            let PendingEpochEventKind::UpdateActiveSet { update } = events[0].kind else {
                panic!("unexpected epoch event")
            };
            assert_eq!(update, issued_update);

            test.execute_all_pending_events();
            let new = storage::REWARDING_PARAMS
                .load(test.deps().storage)
                .unwrap()
                .active_set_size();
            assert_ne!(old, new);
            assert_eq!(new, expected_updated);
        }
    }

    #[cfg(test)]
    mod updating_rewarding_params {
        use cosmwasm_std::Decimal;
        use cw_controllers::AdminError::NotAdmin;

        use mixnet_contract_common::nym_node::Role;
        use mixnet_contract_common::reward_params::RewardedSetParams;
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
                EpochState::RoleAssignment {
                    next: Role::first(),
                },
            ];

            let update = IntervalRewardingParamsUpdate {
                reward_pool: None,
                staking_supply: None,
                staking_supply_scale_factor: None,
                sybil_resistance_percent: None,
                active_set_work_factor: None,
                interval_pool_emission: None,
                rewarded_set_params: Some(RewardedSetParams {
                    entry_gateways: 123,
                    exit_gateways: 123,
                    mixnodes: 300,
                    standby: 123,
                }),
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
            let random = message_info(&test.make_addr("random-guy"), &[]);

            let update = IntervalRewardingParamsUpdate {
                reward_pool: None,
                staking_supply: None,
                staking_supply_scale_factor: None,
                sybil_resistance_percent: None,
                active_set_work_factor: None,
                interval_pool_emission: None,
                rewarded_set_params: Some(RewardedSetParams {
                    entry_gateways: 123,
                    exit_gateways: 123,
                    mixnodes: 300,
                    standby: 123,
                }),
            };

            let env = test.env();
            let res = try_update_rewarding_params(
                test.deps_mut(),
                env.clone(),
                rewarding_validator,
                update,
                false,
            );
            assert_eq!(res, Err(MixnetContractError::Admin(NotAdmin {})));

            let res =
                try_update_rewarding_params(test.deps_mut(), env.clone(), random, update, false);
            assert_eq!(res, Err(MixnetContractError::Admin(NotAdmin {})));

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
                rewarded_set_params: None,
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
                rewarded_set_params: Some(RewardedSetParams {
                    entry_gateways: 123,
                    exit_gateways: 123,
                    mixnodes: 300,
                    standby: 123,
                }),
            };

            let old = storage::REWARDING_PARAMS.load(test.deps().storage).unwrap();
            test.skip_to_current_interval_end();

            let env = test.env();
            let res =
                try_update_rewarding_params(test.deps_mut(), env, owner.clone(), update, false);
            assert!(res.is_ok());
            let new = storage::REWARDING_PARAMS.load(test.deps().storage).unwrap();
            assert_ne!(old, new);
            assert_eq!(new.rewarded_set_size(), 669);

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
                rewarded_set_params: Some(RewardedSetParams {
                    entry_gateways: 123,
                    exit_gateways: 123,
                    mixnodes: 300,
                    standby: 123,
                }),
            };

            let old = storage::REWARDING_PARAMS.load(test.deps().storage).unwrap();
            let env = test.env();
            let res = try_update_rewarding_params(test.deps_mut(), env, owner, update, true);
            assert!(res.is_ok());
            let new = storage::REWARDING_PARAMS.load(test.deps().storage).unwrap();
            assert_ne!(old, new);
            assert_eq!(new.rewarded_set_size(), 669);
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
                rewarded_set_params: Some(RewardedSetParams {
                    entry_gateways: 123,
                    exit_gateways: 123,
                    mixnodes: 300,
                    standby: 123,
                }),
            };

            let old = storage::REWARDING_PARAMS.load(test.deps().storage).unwrap();
            let env = test.env();
            let res = try_update_rewarding_params(test.deps_mut(), env, owner, update, false);
            assert!(res.is_ok());
            let new = storage::REWARDING_PARAMS.load(test.deps().storage).unwrap();
            assert_eq!(old, new);

            // make sure it's actually saved to pending events
            let events = test.pending_interval_events();
            let PendingIntervalEventKind::UpdateRewardingParams { update } = events[0].kind else {
                panic!("unexpected epoch event")
            };
            let Some(rewarded_set_update) = update.rewarded_set_params else {
                panic!("no rewarded set updates");
            };
            assert_eq!(rewarded_set_update.rewarded_set_size(), 669);

            test.execute_all_pending_events();
            let new = storage::REWARDING_PARAMS.load(test.deps().storage).unwrap();
            assert_ne!(old, new);
            assert_eq!(new.rewarded_set_size(), 669);
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
                rewarded_set_params: None,
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

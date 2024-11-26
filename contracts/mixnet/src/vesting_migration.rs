// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::delegations::storage as delegations_storage;
use crate::mixnet_contract_settings::storage as mixnet_params_storage;
use crate::mixnodes::helpers::get_mixnode_details_by_owner;
use crate::mixnodes::storage as mixnodes_storage;
use crate::rewards::storage as rewards_storage;
use crate::support::helpers::{
    ensure_bonded, ensure_epoch_in_progress_state, ensure_no_pending_pledge_changes,
};
use cosmwasm_std::{wasm_execute, DepsMut, Env, Event, MessageInfo, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::{Delegation, NodeId};
use vesting_contract_common::messages::ExecuteMsg as VestingExecuteMsg;

pub(crate) fn try_migrate_vested_mixnode(
    deps: DepsMut<'_>,
    info: MessageInfo,
) -> Result<Response, MixnetContractError> {
    let mix_details = get_mixnode_details_by_owner(deps.storage, info.sender.clone())?.ok_or(
        MixnetContractError::NoAssociatedMixNodeBond {
            owner: info.sender.clone(),
        },
    )?;
    let mix_id = mix_details.mix_id();

    ensure_epoch_in_progress_state(deps.storage)?;
    ensure_no_pending_pledge_changes(&mix_details.pending_changes)?;
    ensure_bonded(&mix_details.bond_information)?;

    let Some(proxy) = &mix_details.bond_information.proxy else {
        return Err(MixnetContractError::NotAVestingMixnode);
    };

    let vesting_contract = mixnet_params_storage::vesting_contract_address(deps.storage)?;
    if proxy != vesting_contract {
        return Err(MixnetContractError::ProxyIsNotVestingContract {
            received: proxy.clone(),
            vesting_contract,
        });
    }

    let mut updated_bond = mix_details.bond_information.clone();
    updated_bond.proxy = None;
    mixnodes_storage::mixnode_bonds().replace(
        deps.storage,
        mix_id,
        Some(&updated_bond),
        Some(&mix_details.bond_information),
    )?;

    Ok(Response::new()
        .add_event(Event::new("migrate-vested-mixnode").add_attribute("mix_id", mix_id.to_string()))
        .add_message(wasm_execute(
            vesting_contract,
            &VestingExecuteMsg::TrackMigratedMixnode {
                owner: info.sender.into_string(),
            },
            vec![],
        )?))
}

pub(crate) fn try_migrate_vested_delegation(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    mix_id: NodeId,
) -> Result<Response, MixnetContractError> {
    let mut response = Response::new();

    ensure_epoch_in_progress_state(deps.storage)?;

    let vesting_contract = mixnet_params_storage::vesting_contract_address(deps.storage)?;

    let storage_key =
        Delegation::generate_storage_key(mix_id, &info.sender, Some(&vesting_contract));
    let Some(vested_delegation) =
        delegations_storage::delegations().may_load(deps.storage, storage_key.clone())?
    else {
        return Err(MixnetContractError::NotAVestingDelegation);
    };

    // sanity check that's meant to blow up the contract
    assert_eq!(vested_delegation.proxy, Some(vesting_contract.clone()));

    // update the delegation and save it under the correct storage key
    let mut updated_delegation = vested_delegation.clone();
    updated_delegation.proxy = None;

    let new_storage_key = Delegation::generate_storage_key(mix_id, &info.sender, None);

    // remove the old (vested) delegation
    delegations_storage::delegations().remove(deps.storage, storage_key)?;

    // check if there was already a delegation present under that key (i.e. an old liquid one)
    if let Some(existing_liquid_delegation) =
        delegations_storage::delegations().may_load(deps.storage, new_storage_key.clone())?
    {
        // treat it as adding extra stake to the existing delegation, so we need to update the unit reward value
        // as well as retrieve any pending rewards
        // it replicates part of code from `pending_events::delegate`,
        // but without some checks that'd be redundant in this instance
        let mut mix_rewarding =
            rewards_storage::MIXNODE_REWARDING.load(deps.storage, vested_delegation.node_id)?;

        // calculate rewards separately for the purposes of emitting those in events
        let pending_liquid_reward =
            mix_rewarding.determine_delegation_reward(&existing_liquid_delegation)?;
        let pending_vested_reward =
            mix_rewarding.determine_delegation_reward(&vested_delegation)?;

        // the calls to 'undelegate' followed by artificial delegate are performed
        // to keep the internal `.delegates` field in sync
        // (this is due to the fact delegation only holds values up in `Uint128` and lacks the precision of a `Decimal`
        // which has to be used for reward accounting)
        let liquid_delegation_with_reward =
            mix_rewarding.undelegate(&existing_liquid_delegation)?;
        let vested_delegation_with_reward = mix_rewarding.undelegate(&vested_delegation)?;

        // updated delegation amount consists of:
        // - delegated vested tokens
        // - delegated liquid tokens
        // - pending rewards earned by the delegated vested tokens
        // - pending rewards earned by the delegated liquid tokens
        let mut updated_total = liquid_delegation_with_reward.clone();
        updated_total.amount += vested_delegation_with_reward.amount;
        mix_rewarding.add_base_delegation(updated_total.amount)?;

        updated_delegation.amount = updated_total;
        updated_delegation.height = env.block.height;
        updated_delegation.cumulative_reward_ratio = mix_rewarding.total_unit_reward;

        rewards_storage::MIXNODE_REWARDING.save(
            deps.storage,
            vested_delegation.node_id,
            &mix_rewarding,
        )?;

        // replace the old delegation with the new one
        delegations_storage::delegations().replace(
            deps.storage,
            new_storage_key,
            Some(&updated_delegation),
            Some(&existing_liquid_delegation),
        )?;

        // just emit EVERYTHING we can. just in case
        response.events.push(
            Event::new("migrate-vested-delegation")
                .add_attribute("mix_id", mix_id.to_string())
                .add_attribute("existing_liquid", "true")
                .add_attribute(
                    "old_vested_unit_reward",
                    vested_delegation.cumulative_reward_ratio.to_string(),
                )
                .add_attribute(
                    "old_vested_delegation_amount",
                    vested_delegation.amount.to_string(),
                )
                .add_attribute(
                    "old_liquid_unit_reward",
                    existing_liquid_delegation
                        .cumulative_reward_ratio
                        .to_string(),
                )
                .add_attribute(
                    "old_liquid_delegation_amount",
                    existing_liquid_delegation.amount.to_string(),
                )
                .add_attribute(
                    "new_unit_reward",
                    updated_delegation.cumulative_reward_ratio.to_string(),
                )
                .add_attribute(
                    "new_delegation_amount",
                    updated_delegation.amount.to_string(),
                )
                .add_attribute("applied_liquid_reward", pending_liquid_reward.to_string())
                .add_attribute("applied_vested_reward", pending_vested_reward.to_string()),
        )
    } else {
        // otherwise, this is as simple as resaving the updated value under the new key
        delegations_storage::delegations().save(
            deps.storage,
            new_storage_key,
            &updated_delegation,
        )?;

        response.events.push(
            Event::new("migrate-vested-delegation")
                .add_attribute("mix_id", mix_id.to_string())
                .add_attribute("existing_liquid", "false")
                .add_attribute(
                    "old_vested_unit_reward",
                    vested_delegation.cumulative_reward_ratio.to_string(),
                )
                .add_attribute(
                    "old_vested_delegation_amount",
                    vested_delegation.amount.to_string(),
                ),
        )
    }

    Ok(response.add_message(wasm_execute(
        vesting_contract,
        &VestingExecuteMsg::TrackMigratedDelegation {
            owner: info.sender.into_string(),
            mix_id,
        },
        vec![],
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod migrating_vested_mixnode {
        use super::*;
        use crate::mixnodes::helpers::get_mixnode_details_by_id;
        use crate::support::tests::test_helpers::TestSetup;
        use cosmwasm_std::testing::mock_info;
        use cosmwasm_std::{from_binary, Addr, CosmosMsg, WasmMsg};

        #[test]
        fn with_no_bonded_nodes() {
            let mut test = TestSetup::new();

            let sender = mock_info("owner", &[]);
            let deps = test.deps_mut();

            // nothing happens
            let res = try_migrate_vested_mixnode(deps, sender).unwrap_err();
            assert_eq!(
                res,
                MixnetContractError::NoAssociatedMixNodeBond {
                    owner: Addr::unchecked("owner")
                }
            )
        }

        #[test]
        fn with_liquid_node_bonded() {
            let mut test = TestSetup::new();
            test.add_legacy_mixnode("owner", None);

            let sender = mock_info("owner", &[]);
            let deps = test.deps_mut();

            // nothing happens
            let res = try_migrate_vested_mixnode(deps, sender).unwrap_err();
            assert_eq!(res, MixnetContractError::NotAVestingMixnode)
        }

        #[test]
        fn with_vested_node_bonded() {
            let mut test = TestSetup::new();
            let mix_id = test.add_legacy_mixnode_with_legal_proxy("owner", None);

            let sender = mock_info("owner", &[]);
            let deps = test.deps_mut();

            let existing_node = get_mixnode_details_by_id(deps.storage, mix_id)
                .unwrap()
                .unwrap();
            assert!(existing_node.bond_information.proxy.is_some());

            let mut expected = existing_node.clone();
            expected.bond_information.proxy = None;

            // node is simply resaved with proxy data removed and a track message is sent into the vesting contract
            let res = try_migrate_vested_mixnode(deps, sender).unwrap();
            let CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) = &res.messages[0].msg else {
                panic!("no track message present")
            };

            assert_eq!(
                from_binary::<VestingExecuteMsg>(msg).unwrap(),
                VestingExecuteMsg::TrackMigratedMixnode {
                    owner: "owner".to_string()
                }
            );
        }
    }

    #[cfg(test)]
    mod migrating_vested_delegation {
        use super::*;
        use crate::delegations::storage::delegations;
        use crate::support::tests::test_helpers::{assert_eq_with_leeway, TestSetup};
        use cosmwasm_std::testing::mock_info;
        use cosmwasm_std::{from_binary, Addr, CosmosMsg, Order, Uint128, WasmMsg};
        use mixnet_contract_common::helpers::compare_decimals;
        use mixnet_contract_common::nym_node::Role;
        use mixnet_contract_common::reward_params::{NodeRewardingParameters, Performance};
        use mixnet_contract_common::rewarding::helpers::truncate_reward;
        use mixnet_contract_common::RoleAssignment;
        use rand::RngCore;

        #[test]
        fn with_no_delegation() {
            let mut test = TestSetup::new_complex();
            let env = test.env();

            let sender = mock_info("owner-without-any-delegations", &[]);

            // it simply fails for there is nothing to migrate
            let res = try_migrate_vested_delegation(test.deps_mut(), env, sender, 42).unwrap_err();
            assert_eq!(res, MixnetContractError::NotAVestingDelegation);
        }

        #[test]
        fn with_just_liquid_delegation() {
            let mut test = TestSetup::new_complex();
            let env = test.env();

            // find a valid delegation
            let delegation = delegations()
                .range(test.deps().storage, None, None, Order::Ascending)
                .filter_map(|d| d.map(|(_, del)| del).ok())
                .find(|d| d.proxy.is_none())
                .unwrap();

            // make sure we haven't chosen somebody that also has a vested delegation because that would have invalidated the test
            assert!(!delegations()
                .range(test.deps().storage, None, None, Order::Ascending)
                .filter_map(|d| d.map(|(_, del)| del).ok())
                .any(|d| d.proxy.is_some() && d.owner.as_str() == delegation.owner.as_str()));

            let sender = mock_info(delegation.owner.as_str(), &[]);
            let mix_id = delegation.node_id;

            // it also fails because the method is only allowed for vested delegations
            let res =
                try_migrate_vested_delegation(test.deps_mut(), env, sender, mix_id).unwrap_err();
            assert_eq!(res, MixnetContractError::NotAVestingDelegation);
        }

        #[test]
        fn with_just_vested_delegation() {
            let mut test = TestSetup::new_complex();
            let env = test.env();

            // find a valid delegation
            let delegation = delegations()
                .range(test.deps().storage, None, None, Order::Ascending)
                .filter_map(|d| d.map(|(_, del)| del).ok())
                .find(|d| d.proxy.is_some())
                .unwrap();

            // make sure we haven't chosen somebody that also has a liquid delegation because that would have invalidated the test
            assert!(!delegations()
                .range(test.deps().storage, None, None, Order::Ascending)
                .filter_map(|d| d.map(|(_, del)| del).ok())
                .any(|d| d.proxy.is_none() && d.owner.as_str() == delegation.owner.as_str()));

            let storage_key = delegation.storage_key();
            let mut expected_liquid = delegation.clone();
            expected_liquid.proxy = None;
            let expected_new_storage_key = expected_liquid.storage_key();

            let sender = mock_info(delegation.owner.as_str(), &[]);
            let mix_id = delegation.node_id;

            //  a track message is sent into the vesting contract
            let res = try_migrate_vested_delegation(test.deps_mut(), env, sender, mix_id).unwrap();
            let CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) = &res.messages[0].msg else {
                panic!("no track message present")
            };

            assert_eq!(
                from_binary::<VestingExecuteMsg>(msg).unwrap(),
                VestingExecuteMsg::TrackMigratedDelegation {
                    owner: delegation.owner.to_string(),
                    mix_id,
                }
            );

            // the entry is gone from the old storage key
            assert!(delegations()
                .may_load(test.deps().storage, storage_key)
                .unwrap()
                .is_none());

            // and is resaved (without proxy) under the new key
            assert_eq!(
                expected_liquid,
                delegations()
                    .load(test.deps().storage, expected_new_storage_key)
                    .unwrap()
            );
        }

        #[test]
        fn with_both_liquid_and_vested_delegation() {
            let mut test = TestSetup::new_complex();
            let env = test.env();

            let problematic_delegator = "n1foomp";
            let problematic_delegator_twin = "n1bar";
            let mix_id = 4;

            let liquid_storage_key = Delegation::generate_storage_key(
                mix_id,
                &Addr::unchecked(problematic_delegator),
                None,
            );
            let vested_storage_key = Delegation::generate_storage_key(
                mix_id,
                &Addr::unchecked(problematic_delegator),
                Some(&test.vesting_contract()),
            );

            let liquid_delegation = delegations()
                .load(test.deps().storage, liquid_storage_key.clone())
                .unwrap();
            let vested_delegation = delegations()
                .load(test.deps().storage, vested_storage_key.clone())
                .unwrap();
            let mix_info = test.mix_rewarding(mix_id);
            let unclaimed_liquid_reward = mix_info
                .determine_delegation_reward(&liquid_delegation)
                .unwrap();
            let unclaimed_vested_reward = mix_info
                .determine_delegation_reward(&vested_delegation)
                .unwrap();

            // sanity check before doing anything
            test.ensure_delegation_sync(mix_id);

            //  a track message is sent into the vesting contract
            let sender = mock_info(problematic_delegator, &[]);
            let res = try_migrate_vested_delegation(test.deps_mut(), env, sender, mix_id).unwrap();
            let CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) = &res.messages[0].msg else {
                panic!("no track message present")
            };

            assert_eq!(
                from_binary::<VestingExecuteMsg>(msg).unwrap(),
                VestingExecuteMsg::TrackMigratedDelegation {
                    owner: problematic_delegator.to_string(),
                    mix_id,
                }
            );

            let updated_mix_info = test.mix_rewarding(mix_id);
            assert_eq!(
                mix_info.unique_delegations - 1,
                updated_mix_info.unique_delegations
            );

            // the vested delegation is gone
            assert!(delegations()
                .may_load(test.deps().storage, vested_storage_key)
                .unwrap()
                .is_none());

            let updated_liquid_delegation = delegations()
                .load(test.deps().storage, liquid_storage_key.clone())
                .unwrap();

            assert!(updated_liquid_delegation.proxy.is_none());
            assert_eq!(
                updated_liquid_delegation.cumulative_reward_ratio,
                updated_mix_info.total_unit_reward
            );

            let expected_amount = truncate_reward(
                vested_delegation.dec_amount().unwrap()
                    + liquid_delegation.dec_amount().unwrap()
                    + unclaimed_liquid_reward
                    + unclaimed_vested_reward,
                "unym",
            );
            // due to rounding we can expect and tolerate a single token of difference
            assert_eq_with_leeway(
                updated_liquid_delegation.amount.amount,
                expected_amount.amount,
                Uint128::one(),
            );

            // this assertion must still hold
            test.ensure_delegation_sync(mix_id);

            // go through few more rewarding epochs to make sure the rewards and accounting
            // would be the same as if the delegations remained separate
            let all_nodes = test.all_mixnodes();

            let twin_liquid_storage_key = Delegation::generate_storage_key(
                mix_id,
                &Addr::unchecked(problematic_delegator_twin),
                None,
            );
            let twin_vested_storage_key = Delegation::generate_storage_key(
                mix_id,
                &Addr::unchecked(problematic_delegator_twin),
                Some(&test.vesting_contract()),
            );

            let twin_liquid_delegation = delegations()
                .load(test.deps().storage, twin_liquid_storage_key.clone())
                .unwrap();
            let twin_vested_delegation = delegations()
                .load(test.deps().storage, twin_vested_storage_key.clone())
                .unwrap();

            let info = test.mix_rewarding(mix_id);

            let unclaimed_rewards_twin_liquid = info
                .determine_delegation_reward(&twin_liquid_delegation)
                .unwrap();
            let unclaimed_rewards_twin_vested = info
                .determine_delegation_reward(&twin_vested_delegation)
                .unwrap();

            for _ in 0..100 {
                test.skip_to_next_epoch_end();
                // it doesn't matter that they're on the same layer here, we just need to make sure they're rewarded
                test.force_assign_rewarded_set(vec![RoleAssignment {
                    role: Role::Layer1,
                    nodes: all_nodes.clone(),
                }]);
                test.start_epoch_transition();

                // reward each node
                for node in &all_nodes {
                    let performance = test.rng.next_u64() % 100;
                    let work_factor = test.active_node_work();
                    test.reward_with_distribution(
                        *node,
                        NodeRewardingParameters {
                            performance: Performance::from_percentage_value(performance).unwrap(),
                            work_factor,
                        },
                    );
                }

                test.set_epoch_in_progress_state();
            }

            // this assertion must still hold
            test.ensure_delegation_sync(mix_id);

            let info = test.mix_rewarding(mix_id);

            let current_liquid = delegations()
                .load(test.deps().storage, liquid_storage_key)
                .unwrap();
            let rewards = info.determine_delegation_reward(&current_liquid).unwrap();

            let twin_liquid_delegation = delegations()
                .load(test.deps().storage, twin_liquid_storage_key.clone())
                .unwrap();
            let twin_vested_delegation = delegations()
                .load(test.deps().storage, twin_vested_storage_key.clone())
                .unwrap();

            let rewards_twin_liquid = info
                .determine_delegation_reward(&twin_liquid_delegation)
                .unwrap();
            let rewards_twin_vested = info
                .determine_delegation_reward(&twin_vested_delegation)
                .unwrap();

            let new_rewards_twin = rewards_twin_liquid + rewards_twin_vested
                - unclaimed_rewards_twin_liquid
                - unclaimed_rewards_twin_vested;

            compare_decimals(rewards, new_rewards_twin, Some("0.01".parse().unwrap()))
        }
    }
}

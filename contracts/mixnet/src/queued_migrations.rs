// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::delegations::storage::delegations;
use crate::rewards::storage::MIXNODE_REWARDING;
use cosmwasm_std::{Addr, Decimal, DepsMut, Env, Event, Order, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::helpers::IntoBaseDecimal;
use mixnet_contract_common::rewarding::helpers::truncate_reward;
use mixnet_contract_common::{AffectedNode, Delegation};
use std::collections::BTreeMap;

fn fix_affected_node(
    response: &mut Response,
    deps: DepsMut<'_>,
    env: &Env,
    node: AffectedNode,
) -> Result<(), MixnetContractError> {
    let total_ratio = node.total_ratio();
    let one = Decimal::one();

    // the total ratio has to be equal to 1 (or be extremely close to it, because it can be affected by rounding)
    // if it doesn't it means we passed an invalid migrate msg and we HAVE TO fail the migration if that's the case
    let epsilon = Decimal::from_ratio(1u128, 100_000_000u128);

    if total_ratio > one {
        if total_ratio - one >= epsilon {
            return Err(MixnetContractError::FailedMigration {
                comment: format!(
                    "the total delegation ratio for node {} does not sum up to 1",
                    node.mix_id
                ),
            });
        }
    } else if one - total_ratio >= epsilon {
        return Err(MixnetContractError::FailedMigration {
            comment: format!(
                "the total delegation ratio for node {} does not sum up to 1",
                node.mix_id
            ),
        });
    }

    let mut total_accounted_for = Decimal::zero();
    let mut mix_rewarding = MIXNODE_REWARDING.load(deps.storage, node.mix_id)?;

    let mut cached_delegations = BTreeMap::new();

    // determine all the stake accounted for, i.e. all delegations and their pending rewards
    for entry in delegations()
        .idx
        .mixnode
        .prefix(node.mix_id)
        .range(deps.storage, None, None, Order::Ascending)
        .map(|record| record.map(|r| r.1))
    {
        let delegation = entry?;
        let base_delegation = delegation.dec_amount()?;
        let pending_reward = mix_rewarding.determine_delegation_reward(&delegation)?;

        // cache the delegation and reward for the lookup in the next loop
        if node
            .delegators
            .iter()
            .any(|d| d.address == delegation.owner.as_str())
        {
            cached_delegations.insert(delegation.owner.to_string(), (delegation, pending_reward));
        }

        total_accounted_for += base_delegation;
        total_accounted_for += pending_reward;
    }

    // sanity check
    assert!(cached_delegations.len() <= node.delegators.len());

    // the missing stake equals to the difference between total node delegation (which includes all rewards, etc.)
    // and the value we managed to just account for
    let node_missing = mix_rewarding.delegates - total_accounted_for;

    let mut distributed = Decimal::zero();

    // finally split the missing stake among the affected delegators according to the ratios
    // provided in the migration which were very painstakingly determined by scraping different
    // sources of chain data
    for delegator in node.delegators {
        let restored = node_missing * delegator.missing_ratio;
        distributed += restored;

        // we have two scenarios to cover here:
        // 1. somebody performed vested migration and then undelegated the tokens (*sigh*)
        //    - in that case we have to create brand-new delegation with the restored amount
        // 2. the delegation still exists
        //    - in that case we have to increase the existing delegation. essentially treat it as if somebody delegated extra tokens

        if let Some((old_liquid_delegation, pending_reward)) =
            cached_delegations.remove(&delegator.address)
        {
            // delegation still exists

            assert!(old_liquid_delegation.proxy.is_none());

            let old_liquid = old_liquid_delegation.dec_amount()? + pending_reward;
            let updated_amount_dec = old_liquid + restored;
            let updated_amount =
                truncate_reward(updated_amount_dec, &old_liquid_delegation.amount.denom);

            // take the truncation into consideration for the purposes of future accounting
            let truncated_delta = updated_amount_dec - updated_amount.amount.into_base_decimal()?;
            mix_rewarding.delegates -= truncated_delta;

            // just emit EVERYTHING we can. just in case
            response.events.push(
                Event::new("delegation_restoration")
                    .add_attribute("delegator", delegator.address)
                    .add_attribute("delegator_ratio", delegator.missing_ratio.to_string())
                    .add_attribute("mix_id", node.mix_id.to_string())
                    .add_attribute("restored_amount_dec", restored.to_string())
                    .add_attribute("node_delegates", mix_rewarding.delegates.to_string())
                    .add_attribute("total_node_delegations", total_accounted_for.to_string())
                    .add_attribute("total_missing_delegations", node_missing.to_string())
                    .add_attribute("updated_amount_dec", updated_amount_dec.to_string())
                    .add_attribute("updated_amount", updated_amount.to_string())
                    .add_attribute("liquid_delegation_existed", "true")
                    .add_attribute(
                        "old_liquid_delegation_unit_reward",
                        old_liquid_delegation.cumulative_reward_ratio.to_string(),
                    )
                    .add_attribute(
                        "old_liquid_delegation_amount",
                        old_liquid_delegation.amount.to_string(),
                    )
                    .add_attribute(
                        "old_liquid_delegation_pending_reward",
                        pending_reward.to_string(),
                    )
                    .add_attribute("truncated_amount", truncated_delta.to_string()),
            );

            // create new delegation with the updated amount
            // and also, what's very important, with correct unit reward amount
            let updated_delegation = Delegation::new(
                old_liquid_delegation.owner.clone(),
                node.mix_id,
                mix_rewarding.total_unit_reward,
                updated_amount,
                env.block.height,
            );

            // replace the value stored under the existing key
            let delegation_storage_key = old_liquid_delegation.storage_key();
            delegations().replace(
                deps.storage,
                delegation_storage_key,
                Some(&updated_delegation),
                Some(&old_liquid_delegation),
            )?;
        } else {
            let restored_amount = truncate_reward(restored, "unym");

            // take the truncation into consideration for the purposes of future accounting
            let truncated_delta = restored - restored_amount.amount.into_base_decimal()?;
            mix_rewarding.delegates -= truncated_delta;

            // delegation is now gone - create a new one with the restored amount
            let delegation = Delegation::new(
                Addr::unchecked(&delegator.address),
                node.mix_id,
                mix_rewarding.total_unit_reward,
                restored_amount,
                env.block.height,
            );

            let delegation_storage_key = delegation.storage_key();
            delegations().save(deps.storage, delegation_storage_key, &delegation)?;

            response.events.push(
                Event::new("delegation_restoration")
                    .add_attribute("delegator", delegator.address)
                    .add_attribute("delegator_ratio", delegator.missing_ratio.to_string())
                    .add_attribute("mix_id", node.mix_id.to_string())
                    .add_attribute("restored_amount_dec", restored.to_string())
                    .add_attribute("node_delegates", mix_rewarding.delegates.to_string())
                    .add_attribute("total_node_delegations", total_accounted_for.to_string())
                    .add_attribute("total_missing_delegations", node_missing.to_string())
                    .add_attribute("updated_amount_dec", restored.to_string())
                    .add_attribute("updated_amount", delegation.amount.to_string())
                    .add_attribute("liquid_delegation_existed", "false")
                    .add_attribute("truncated_amount", truncated_delta.to_string()),
            );
        }

        // the vested and liquid delegations got combined into one
        mix_rewarding.unique_delegations -= 1;
        MIXNODE_REWARDING.save(deps.storage, node.mix_id, &mix_rewarding)?;
    }

    response.events.push(
        Event::new("node_delegation_restoration")
            .add_attribute("mix_id", node.mix_id.to_string())
            .add_attribute("node_delegates", mix_rewarding.delegates.to_string())
            .add_attribute("total_node_delegations", total_accounted_for.to_string())
            .add_attribute("total_missing_delegations", node_missing.to_string())
            .add_attribute("total_redistributed", distributed.to_string()),
    );

    // another sanity check
    assert!(distributed <= node_missing);
    Ok(())
}

pub fn restore_vested_delegations(
    response: &mut Response,
    mut deps: DepsMut<'_>,
    env: Env,
    affected_nodes: Vec<AffectedNode>,
) -> Result<(), MixnetContractError> {
    for node in affected_nodes {
        fix_affected_node(response, deps.branch(), &env, node)?
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod restoring_vested_delegations {
        use super::*;
        use crate::support::tests::test_helpers::{assert_eq_with_leeway, TestSetup};
        use crate::vesting_migration::try_migrate_vested_delegation;
        use cosmwasm_std::testing::mock_info;
        use cosmwasm_std::Uint128;
        use mixnet_contract_common::reward_params::Performance;
        use mixnet_contract_common::rewarding::helpers::truncate_reward_amount;
        use mixnet_contract_common::AffectedDelegator;
        use nym_contracts_common::truncate_decimal;
        use rand::RngCore;

        #[test]
        fn for_node_with_single_affected_delegator_without_undelegating() {
            let mut test = TestSetup::new_complex();

            let problematic_delegator = "n1foomp";
            let problematic_delegator_twin = "n1bar";
            let mix_id = 4;

            // "accidentally" overwrite the delegation
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
            let vested_delegation = delegations()
                .load(test.deps().storage, vested_storage_key.clone())
                .unwrap();
            let mut bad_liquid_delegation = vested_delegation.clone();
            bad_liquid_delegation.proxy = None;

            delegations()
                .remove(test.deps_mut().storage, vested_storage_key)
                .unwrap();
            delegations()
                .save(
                    test.deps_mut().storage,
                    liquid_storage_key,
                    &bad_liquid_delegation,
                )
                .unwrap();

            // go through few rewarding cycles...
            let all_nodes = test.all_mixnodes();
            for _ in 0..100 {
                test.skip_to_next_epoch_end();
                test.force_change_rewarded_set(all_nodes.clone());
                test.start_epoch_transition();

                // reward each node
                for node in &all_nodes {
                    let performance = test.rng.next_u64() % 100;
                    test.reward_with_distribution(
                        *node,
                        Performance::from_percentage_value(performance).unwrap(),
                    );
                }

                test.set_epoch_in_progress_state();
            }

            // restoring problematic delegator should be equivalent to the delegator twin just migrating
            let env = test.env();
            fix_affected_node(
                &mut Response::new(),
                test.deps_mut(),
                &env,
                AffectedNode {
                    mix_id,
                    delegators: vec![AffectedDelegator {
                        address: problematic_delegator.to_string(),
                        missing_ratio: Decimal::one(),
                    }],
                },
            )
            .unwrap();

            try_migrate_vested_delegation(
                test.deps_mut(),
                env,
                mock_info(problematic_delegator_twin, &[]),
                mix_id,
            )
            .unwrap();

            let liquid_storage_key = Delegation::generate_storage_key(
                mix_id,
                &Addr::unchecked(problematic_delegator),
                None,
            );
            let liquid_storage_key_twin = Delegation::generate_storage_key(
                mix_id,
                &Addr::unchecked(problematic_delegator_twin),
                None,
            );

            let liquid_delegation = delegations()
                .load(test.deps().storage, liquid_storage_key)
                .unwrap();
            let liquid_delegation_alt = delegations()
                .load(test.deps().storage, liquid_storage_key_twin)
                .unwrap();
            assert_eq!(
                liquid_delegation.cumulative_reward_ratio,
                liquid_delegation_alt.cumulative_reward_ratio
            );
            assert_eq_with_leeway(
                liquid_delegation.amount.amount,
                liquid_delegation_alt.amount.amount,
                Uint128::one(),
            );
        }

        #[test]
        fn for_node_with_single_affected_delegator_after_undelegating() {
            let mut test = TestSetup::new_complex();

            let problematic_delegator = "n1foomp";
            let problematic_delegator_twin = "n1bar";
            let mix_id = 4;

            // "accidentally" overwrite the delegation
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
            let vested_delegation = delegations()
                .load(test.deps().storage, vested_storage_key.clone())
                .unwrap();
            let mut bad_liquid_delegation = vested_delegation.clone();
            bad_liquid_delegation.proxy = None;

            delegations()
                .remove(test.deps_mut().storage, vested_storage_key)
                .unwrap();
            delegations()
                .save(
                    test.deps_mut().storage,
                    liquid_storage_key,
                    &bad_liquid_delegation,
                )
                .unwrap();

            // go through few rewarding cycles...
            let all_nodes = test.all_mixnodes();
            for _ in 0..100 {
                test.skip_to_next_epoch_end();
                test.force_change_rewarded_set(all_nodes.clone());
                test.start_epoch_transition();

                // reward each node
                for node in &all_nodes {
                    let performance = test.rng.next_u64() % 100;
                    test.reward_with_distribution(
                        *node,
                        Performance::from_percentage_value(performance).unwrap(),
                    );
                }

                test.set_epoch_in_progress_state();
            }

            // they got scared and undelegated (the removed part is their vested delegation)
            test.remove_immediate_delegation(problematic_delegator, mix_id);

            // go through some more rewarding
            for _ in 0..100 {
                test.skip_to_next_epoch_end();
                test.force_change_rewarded_set(all_nodes.clone());
                test.start_epoch_transition();

                // reward each node
                for node in &all_nodes {
                    let performance = test.rng.next_u64() % 100;
                    test.reward_with_distribution(
                        *node,
                        Performance::from_percentage_value(performance).unwrap(),
                    );
                }

                test.set_epoch_in_progress_state();
            }

            // the restored amount should be equivalent to the liquid part (+ rewards) of the twin delegator
            let env = test.env();
            fix_affected_node(
                &mut Response::new(),
                test.deps_mut(),
                &env,
                AffectedNode {
                    mix_id,
                    delegators: vec![AffectedDelegator {
                        address: problematic_delegator.to_string(),
                        missing_ratio: Decimal::one(),
                    }],
                },
            )
            .unwrap();

            let liquid_storage_key = Delegation::generate_storage_key(
                mix_id,
                &Addr::unchecked(problematic_delegator),
                None,
            );
            let liquid_storage_key_twin = Delegation::generate_storage_key(
                mix_id,
                &Addr::unchecked(problematic_delegator_twin),
                None,
            );

            let liquid_delegation = delegations()
                .load(test.deps().storage, liquid_storage_key)
                .unwrap();
            let liquid_delegation_alt = delegations()
                .load(test.deps().storage, liquid_storage_key_twin)
                .unwrap();
            let mix_info = test.mix_rewarding(mix_id);
            let pending_twin_reward = mix_info
                .determine_delegation_reward(&liquid_delegation_alt)
                .unwrap();

            assert_eq!(
                liquid_delegation.cumulative_reward_ratio,
                mix_info.total_unit_reward
            );
            assert_eq_with_leeway(
                liquid_delegation.amount.amount,
                liquid_delegation_alt.amount.amount + truncate_reward_amount(pending_twin_reward),
                Uint128::one(),
            );
        }

        #[test]
        fn for_node_with_multiple_affected_delegators() {
            let mut test = TestSetup::new_complex();

            // some random delegator
            let problematic_delegator = "n1foomp";

            // another delegator that made DIFFERENT delegations as the previous ones BUT to the same node
            let problematic_delegator_alt_twin = "n1whatever";

            let mix_id = 4;
            let mix_info_start = test.mix_rewarding(mix_id);

            // "accidentally" overwrite the delegations
            let liquid_storage_key1 = Delegation::generate_storage_key(
                mix_id,
                &Addr::unchecked(problematic_delegator),
                None,
            );
            let vested_storage_key1 = Delegation::generate_storage_key(
                mix_id,
                &Addr::unchecked(problematic_delegator),
                Some(&test.vesting_contract()),
            );
            let liquid_delegation1 = delegations()
                .load(test.deps().storage, liquid_storage_key1.clone())
                .unwrap();
            let vested_delegation1 = delegations()
                .load(test.deps().storage, vested_storage_key1.clone())
                .unwrap();

            // keep track of the 'lost' tokens for test assertions
            let lost1 = liquid_delegation1.dec_amount().unwrap()
                + mix_info_start
                    .determine_delegation_reward(&liquid_delegation1)
                    .unwrap();

            let mut bad_liquid_delegation1 = vested_delegation1.clone();
            bad_liquid_delegation1.proxy = None;

            delegations()
                .remove(test.deps_mut().storage, vested_storage_key1)
                .unwrap();
            delegations()
                .save(
                    test.deps_mut().storage,
                    liquid_storage_key1.clone(),
                    &bad_liquid_delegation1,
                )
                .unwrap();

            let liquid_storage_key2 = Delegation::generate_storage_key(
                mix_id,
                &Addr::unchecked(problematic_delegator_alt_twin),
                None,
            );
            let vested_storage_key2 = Delegation::generate_storage_key(
                mix_id,
                &Addr::unchecked(problematic_delegator_alt_twin),
                Some(&test.vesting_contract()),
            );
            let liquid_delegation2 = delegations()
                .load(test.deps().storage, liquid_storage_key2.clone())
                .unwrap();
            let vested_delegation2 = delegations()
                .load(test.deps().storage, vested_storage_key2.clone())
                .unwrap();
            let lost2 = liquid_delegation2.dec_amount().unwrap()
                + mix_info_start
                    .determine_delegation_reward(&liquid_delegation2)
                    .unwrap();

            let mut bad_liquid_delegation2 = vested_delegation2.clone();
            bad_liquid_delegation2.proxy = None;

            delegations()
                .remove(test.deps_mut().storage, vested_storage_key2)
                .unwrap();
            delegations()
                .save(
                    test.deps_mut().storage,
                    liquid_storage_key2.clone(),
                    &bad_liquid_delegation2,
                )
                .unwrap();

            // go through few rewarding cycles...
            let all_nodes = test.all_mixnodes();

            for _ in 0..100 {
                test.skip_to_next_epoch_end();
                test.force_change_rewarded_set(all_nodes.clone());
                test.start_epoch_transition();

                // reward each node
                for node in &all_nodes {
                    let performance = test.rng.next_u64() % 100;
                    test.reward_with_distribution(
                        *node,
                        Performance::from_percentage_value(performance).unwrap(),
                    );
                }

                test.set_epoch_in_progress_state();
            }

            // those ratios got determined externally. in this test we unfortunately use purely artificial values
            let ratio1: Decimal = "0.45326524362".parse().unwrap();
            let ratio2 = Decimal::one() - ratio1;

            let mix_info = test.mix_rewarding(mix_id);
            let liquid_delegation_before = delegations()
                .load(test.deps().storage, liquid_storage_key1.clone())
                .unwrap();
            let liquid_reward_before = mix_info
                .determine_delegation_reward(&liquid_delegation_before)
                .unwrap();

            let liquid_delegation_alt_before = delegations()
                .load(test.deps().storage, liquid_storage_key2.clone())
                .unwrap();
            let liquid_reward_alt_before = mix_info
                .determine_delegation_reward(&liquid_delegation_alt_before)
                .unwrap();

            let env = test.env();
            let mut res = Response::new();
            fix_affected_node(
                &mut res,
                test.deps_mut(),
                &env,
                AffectedNode {
                    mix_id,
                    delegators: vec![
                        AffectedDelegator {
                            address: problematic_delegator.to_string(),
                            missing_ratio: ratio1,
                        },
                        AffectedDelegator {
                            address: problematic_delegator_alt_twin.to_string(),
                            missing_ratio: ratio2,
                        },
                    ],
                },
            )
            .unwrap();

            let liquid_delegation = delegations()
                .load(test.deps().storage, liquid_storage_key1)
                .unwrap();
            let liquid_delegation_alt = delegations()
                .load(test.deps().storage, liquid_storage_key2)
                .unwrap();

            // the total amount recovered must be equal to what has been lost (approximately)
            let total_lost = lost1 + lost2;
            // determine the compounded rewards on the lost tokens
            // (just unroll `MixNodeRewarding::determine_delegation_reward(...)`)
            let starting_ratio = mix_info_start.total_unit_reward;
            let ending_ratio = mix_info.full_reward_ratio();
            let adjust = starting_ratio + mix_info.unit_delegation;
            let compounded_lost_reward = (ending_ratio - starting_ratio) * total_lost / adjust;

            let before = liquid_delegation_before.dec_amount().unwrap()
                + liquid_delegation_alt_before.dec_amount().unwrap()
                + liquid_reward_before
                + liquid_reward_alt_before;

            let after = liquid_delegation.amount.amount + liquid_delegation_alt.amount.amount;
            let expected_before = truncate_decimal(total_lost + compounded_lost_reward + before);

            assert_eq_with_leeway(after, expected_before, Uint128::one());

            test.ensure_delegation_sync(mix_id);

            // more rewarding
            for _ in 0..100 {
                test.skip_to_next_epoch_end();
                test.force_change_rewarded_set(all_nodes.clone());
                test.start_epoch_transition();

                // reward each node
                for node in &all_nodes {
                    let performance = test.rng.next_u64() % 100;
                    test.reward_with_distribution(
                        *node,
                        Performance::from_percentage_value(performance).unwrap(),
                    );
                }

                test.set_epoch_in_progress_state();
            }

            test.ensure_delegation_sync(mix_id);
        }
    }
}

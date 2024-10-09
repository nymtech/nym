// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::delegations::storage::delegations;
use crate::rewards::storage::MIXNODE_REWARDING;
use cosmwasm_std::{Addr, Decimal, DepsMut, Env, Event, Order, Response};
use mixnet_contract_common::error::MixnetContractError;
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
                    ),
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
            // delegation is now gone - create a new one with the restored amount
            let delegation = Delegation::new(
                Addr::unchecked(&delegator.address),
                node.mix_id,
                mix_rewarding.total_unit_reward,
                truncate_reward(restored, "unym"),
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
                    .add_attribute("liquid_delegation_existed", "false"),
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

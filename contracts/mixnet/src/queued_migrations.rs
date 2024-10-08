// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::delegations::storage::delegations;
use crate::rewards::storage::MIXNODE_REWARDING;
use cosmwasm_std::{Decimal, DepsMut, Env, Event, Order, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::rewarding::helpers::truncate_reward;
use mixnet_contract_common::{AffectedNode, Delegation};

fn fix_affected_node(
    response: &mut Response,
    deps: DepsMut<'_>,
    env: &Env,
    node: AffectedNode,
) -> Result<(), MixnetContractError> {
    let total_ratio = node.total_ratio();
    let one = Decimal::one();

    // the total ratio has to be equal to 1 (or be extremely close to it, because it can be affected by rounding)
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

    // keep cache of entries we're interested in because we'll have to update them
    // (keep it in a vec since the hashmap overhead is actually larger for the few entries we're dealing with)
    let mut cached_delegations = Vec::new();

    // determine the total missing stake
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

        if node
            .delegators
            .iter()
            .any(|d| d.address == delegation.owner.as_str())
        {
            cached_delegations.push((delegation, pending_reward));
        }

        total_accounted_for += base_delegation;
        total_accounted_for += pending_reward;
    }

    // sanity check assertion (we WANT TO blow up if it doesn't hold)
    assert_eq!(cached_delegations.len(), node.delegators.len());

    let node_missing = mix_rewarding.delegates - total_accounted_for;

    let mut distributed = Decimal::zero();

    // finally split the missing stake among the affected delegators
    for delegator in node.delegators {
        // I really hope the affected people haven't attempted undelegating their tokens...
        #[allow(clippy::unwrap_used)]
        let (delegation, pending_reward) = cached_delegations
            .iter()
            .find(|d| d.0.owner.as_str() == delegator.address)
            .unwrap();

        assert!(delegation.proxy.is_none());

        let old_liquid = delegation.dec_amount()? + pending_reward;
        let restored = node_missing * delegator.missing_ratio;
        let updated_amount_dec = old_liquid + restored;
        let updated_amount = truncate_reward(updated_amount_dec, &delegation.amount.denom);

        distributed += restored;

        // just emit EVERYTHING we can. just in case
        response.events.push(
            Event::new("delegation_restoration")
                .add_attribute("delegator", delegator.address)
                .add_attribute("delegator_ratio", delegator.missing_ratio.to_string())
                .add_attribute("mix_id", node.mix_id.to_string())
                .add_attribute("node_delegates", mix_rewarding.delegates.to_string())
                .add_attribute("total_node_delegations", total_accounted_for.to_string())
                .add_attribute("total_missing_delegations", node_missing.to_string())
                .add_attribute("restored_amount_dec", updated_amount_dec.to_string())
                .add_attribute("restored_amount", updated_amount.to_string()),
        );

        let updated_delegation = Delegation::new(
            delegation.owner.clone(),
            node.mix_id,
            mix_rewarding.total_unit_reward,
            updated_amount,
            env.block.height,
        );

        let delegation_storage_key = delegation.storage_key();
        delegations().replace(
            deps.storage,
            delegation_storage_key,
            Some(&updated_delegation),
            Some(delegation),
        )?;

        // the vested and liquid delegations got combined into one
        mix_rewarding.unique_delegations -= 1;
        MIXNODE_REWARDING.save(deps.storage, node.mix_id, &mix_rewarding)?;
    }

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

// Copyright 2022-2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::delegations::storage::delegations;
use crate::rewards::storage::NYMNODE_REWARDING;
use cosmwasm_std::{Decimal, DepsMut, Event, Order, Response};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::NodeId;

/// nodes whose `unique_delegations` was decremented twice by the 2024 vested-delegation
/// restoration: once by the owner's earlier undelegate and once by the restoration itself
pub const NODES_WITH_UNDERCOUNTED_DELEGATIONS: [NodeId; 7] = [26, 106, 108, 193, 255, 323, 455];

/// rounding dust tolerated between `delegates` and the sum of its delegations with pending rewards
const DELEGATES_DUST_TOLERANCE: Decimal = Decimal::one();

pub(crate) fn resync_delegation_counter(
    response: &mut Response,
    deps: DepsMut<'_>,
    node_id: NodeId,
) -> Result<(), MixnetContractError> {
    let Some(mut rewarding) = NYMNODE_REWARDING.may_load(deps.storage, node_id)? else {
        return Err(MixnetContractError::FailedMigration {
            comment: format!("node {node_id}: rewarding record is missing"),
        });
    };

    // go through every delegation of this node...
    let mut entries: u32 = 0;
    let mut accounted_for = Decimal::zero();
    for record in delegations()
        .prefix(node_id)
        .range(deps.storage, None, None, Order::Ascending)
    {
        let (_, delegation) = record?;
        entries += 1;
        accounted_for += delegation.dec_amount()?;
        accounted_for += rewarding.determine_delegation_reward(&delegation)?;
    }

    // stake that has already left the entries (a fired sweep) cannot be repaired by a recount
    // hopefully we won't hit it (as of writing this migration, it's not the case)
    // otherwise the fix up will be a tad more complex...
    if rewarding.delegates.abs_diff(accounted_for) > DELEGATES_DUST_TOLERANCE {
        return Err(MixnetContractError::FailedMigration {
            comment: format!(
                "node {node_id}: delegates {} do not match the {accounted_for} accounted for by its {entries} delegations",
                rewarding.delegates
            ),
        });
    }

    let before = rewarding.unique_delegations;
    let event = Event::new("delegation_counter_resync")
        .add_attribute("node_id", node_id.to_string())
        .add_attribute("counter_before", before.to_string())
        .add_attribute("entries", entries.to_string())
        .add_attribute("delegates", rewarding.delegates.to_string())
        .add_attribute("accounted_for", accounted_for.to_string());

    if before == entries {
        response
            .events
            .push(event.add_attribute("result", "already_consistent"));
        return Ok(());
    }

    // only the measured shape is corrected; anything else is a different problem
    if before + 1 != entries {
        return Err(MixnetContractError::FailedMigration {
            comment: format!(
                "node {node_id}: counter {before} against {entries} delegations is not the expected undercount by one"
            ),
        });
    }

    rewarding.unique_delegations = entries;
    NYMNODE_REWARDING.save(deps.storage, node_id, &rewarding)?;
    response.events.push(
        event
            .add_attribute("result", "fixed")
            .add_attribute("counter_after", entries.to_string()),
    );
    Ok(())
}

/// Recounts `unique_delegations` on the listed nodes; idempotent, fails on any unexpected shape.
pub fn resync_delegation_counters(mut deps: DepsMut<'_>) -> Result<Response, MixnetContractError> {
    let mut response = Response::new();
    for &node_id in &NODES_WITH_UNDERCOUNTED_DELEGATIONS {
        resync_delegation_counter(&mut response, deps.branch(), node_id)?;
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::test_helpers::TestSetup;
    use cosmwasm_std::Uint128;

    const DELEGATION: u128 = 1_000_000_000;

    fn node_with_delegators(test: &mut TestSetup, delegators: u32) -> NodeId {
        let owner = test.random_address();
        let node_id = test.add_dummy_nymnode(&owner, None);
        for i in 0..delegators {
            let delegator = test.make_addr(format!("delegator{i}"));
            test.add_immediate_delegation(&delegator, Uint128::new(DELEGATION), node_id);
        }
        node_id
    }

    fn force_counter(test: &mut TestSetup, node_id: NodeId, counter: u32) {
        let mut rewarding = test.mix_rewarding(node_id);
        rewarding.unique_delegations = counter;
        NYMNODE_REWARDING
            .save(test.deps_mut().storage, node_id, &rewarding)
            .unwrap();
    }

    fn resync_events(response: &Response) -> Vec<&Event> {
        response
            .events
            .iter()
            .filter(|e| e.ty == "delegation_counter_resync")
            .collect()
    }

    fn attribute<'a>(event: &'a Event, key: &str) -> &'a str {
        event
            .attributes
            .iter()
            .find(|a| a.key == key)
            .map(|a| a.value.as_str())
            .unwrap_or_else(|| panic!("missing attribute {key}"))
    }

    #[test]
    fn recounts_a_counter_one_below_the_entries() {
        let mut test = TestSetup::new();
        let node_id = node_with_delegators(&mut test, 3);
        force_counter(&mut test, node_id, 2);

        let mut response = Response::new();
        resync_delegation_counter(&mut response, test.deps_mut(), node_id).unwrap();

        assert_eq!(test.mix_rewarding(node_id).unique_delegations, 3);
        test.ensure_delegation_sync(node_id);

        let events = resync_events(&response);
        assert_eq!(events.len(), 1);
        assert_eq!(attribute(events[0], "node_id"), node_id.to_string());
        assert_eq!(attribute(events[0], "result"), "fixed");
        assert_eq!(attribute(events[0], "counter_before"), "2");
        assert_eq!(attribute(events[0], "counter_after"), "3");
    }

    #[test]
    fn stops_the_last_delegator_from_being_swept_into_the_operator() {
        let mut test = TestSetup::new();
        let node_id = node_with_delegators(&mut test, 2);
        force_counter(&mut test, node_id, 1);
        let operator_before = test.mix_rewarding(node_id).operator;
        let first = test.make_addr("delegator0");
        let last = test.make_addr("delegator1");

        resync_delegation_counter(&mut Response::new(), test.deps_mut(), node_id).unwrap();

        test.remove_immediate_delegation(&first, node_id);
        test.remove_immediate_delegation(&last, node_id);

        let rewarding = test.mix_rewarding(node_id);
        assert_eq!(rewarding.operator, operator_before);
        assert_eq!(rewarding.delegates, Decimal::zero());
        assert_eq!(rewarding.unique_delegations, 0);
    }

    #[test]
    fn leaves_a_consistent_node_untouched() {
        let mut test = TestSetup::new();
        let node_id = node_with_delegators(&mut test, 3);
        let before = test.mix_rewarding(node_id);

        let mut response = Response::new();
        resync_delegation_counter(&mut response, test.deps_mut(), node_id).unwrap();

        assert_eq!(test.mix_rewarding(node_id), before);
        let events = resync_events(&response);
        assert_eq!(events.len(), 1);
        assert_eq!(attribute(events[0], "result"), "already_consistent");
    }

    #[test]
    fn fails_when_the_delegates_no_longer_match_the_entries() {
        // the shape a sweep leaves behind: an entry still holds stake but delegates was zeroed
        let mut test = TestSetup::new();
        let node_id = node_with_delegators(&mut test, 2);
        let mut rewarding = test.mix_rewarding(node_id);
        rewarding.unique_delegations = 1;
        rewarding.delegates = Decimal::zero();
        NYMNODE_REWARDING
            .save(test.deps_mut().storage, node_id, &rewarding)
            .unwrap();

        let err =
            resync_delegation_counter(&mut Response::new(), test.deps_mut(), node_id).unwrap_err();
        assert!(matches!(err, MixnetContractError::FailedMigration { .. }));
    }

    #[test]
    fn fails_when_the_counter_is_short_by_more_than_one() {
        let mut test = TestSetup::new();
        let node_id = node_with_delegators(&mut test, 3);
        force_counter(&mut test, node_id, 1);

        let err =
            resync_delegation_counter(&mut Response::new(), test.deps_mut(), node_id).unwrap_err();
        assert!(matches!(err, MixnetContractError::FailedMigration { .. }));
    }

    #[test]
    fn fails_when_the_counter_exceeds_the_entries() {
        let mut test = TestSetup::new();
        let node_id = node_with_delegators(&mut test, 3);
        force_counter(&mut test, node_id, 4);

        let err =
            resync_delegation_counter(&mut Response::new(), test.deps_mut(), node_id).unwrap_err();
        assert!(matches!(err, MixnetContractError::FailedMigration { .. }));
    }

    #[test]
    fn fails_when_the_rewarding_record_is_missing() {
        let mut test = TestSetup::new();

        let err = resync_delegation_counter(&mut Response::new(), test.deps_mut(), 42).unwrap_err();
        assert!(matches!(err, MixnetContractError::FailedMigration { .. }));
    }
}

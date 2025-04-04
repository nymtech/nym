// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{
    CURRENT_EPOCH_STATUS_KEY, CURRENT_INTERVAL_KEY, EPOCH_EVENT_ID_COUNTER_KEY,
    INTERVAL_EVENT_ID_COUNTER_KEY, LAST_EPOCH_EVENT_ID_KEY, LAST_INTERVAL_EVENT_ID_KEY,
    PENDING_EPOCH_EVENTS_NAMESPACE, PENDING_INTERVAL_EVENTS_NAMESPACE,
};
use cosmwasm_std::{Addr, Env, StdResult, Storage};
use cw_storage_plus::{Item, Map};
use mixnet_contract_common::pending_events::{
    PendingEpochEventData, PendingEpochEventKind, PendingIntervalEventData,
};
use mixnet_contract_common::{
    EpochEventId, EpochStatus, Interval, IntervalEventId, PendingIntervalEventKind,
};

pub(crate) const CURRENT_EPOCH_STATUS: Item<EpochStatus> = Item::new(CURRENT_EPOCH_STATUS_KEY);
pub(crate) const CURRENT_INTERVAL: Item<Interval> = Item::new(CURRENT_INTERVAL_KEY);

pub(crate) const EPOCH_EVENT_ID_COUNTER: Item<EpochEventId> = Item::new(EPOCH_EVENT_ID_COUNTER_KEY);
pub(crate) const INTERVAL_EVENT_ID_COUNTER: Item<IntervalEventId> =
    Item::new(INTERVAL_EVENT_ID_COUNTER_KEY);

pub(crate) const LAST_PROCESSED_EPOCH_EVENT: Item<EpochEventId> =
    Item::new(LAST_EPOCH_EVENT_ID_KEY);
pub(crate) const LAST_PROCESSED_INTERVAL_EVENT: Item<IntervalEventId> =
    Item::new(LAST_INTERVAL_EVENT_ID_KEY);

// we're indexing the events by an increasing ID so that we'd execute them in the order they were created
// (we can't use block height as it's very possible multiple requests might be created in the same block height,
// and composite keys would be more complex than just using an increasing ID)
/// Contains operations that should get resolved at the end of the current epoch.
pub(crate) const PENDING_EPOCH_EVENTS: Map<EpochEventId, PendingEpochEventData> =
    Map::new(PENDING_EPOCH_EVENTS_NAMESPACE);

/// Contains operations that should get resolved at the end of the current interval.
pub(crate) const PENDING_INTERVAL_EVENTS: Map<IntervalEventId, PendingIntervalEventData> =
    Map::new(PENDING_INTERVAL_EVENTS_NAMESPACE);

pub(crate) fn current_epoch_status(storage: &dyn Storage) -> StdResult<EpochStatus> {
    CURRENT_EPOCH_STATUS.load(storage)
}

pub(crate) fn save_current_epoch_status(
    storage: &mut dyn Storage,
    status: &EpochStatus,
) -> StdResult<()> {
    CURRENT_EPOCH_STATUS.save(storage, status)
}

pub(crate) fn current_interval(storage: &dyn Storage) -> StdResult<Interval> {
    CURRENT_INTERVAL.load(storage)
}

pub(crate) fn save_interval(storage: &mut dyn Storage, interval: &Interval) -> StdResult<()> {
    CURRENT_INTERVAL.save(storage, interval)
}

pub(crate) fn next_epoch_event_id_counter(store: &mut dyn Storage) -> StdResult<EpochEventId> {
    let id: EpochEventId = EPOCH_EVENT_ID_COUNTER.may_load(store)?.unwrap_or_default() + 1;
    EPOCH_EVENT_ID_COUNTER.save(store, &id)?;
    Ok(id)
}

pub(crate) fn next_interval_event_id_counter(
    store: &mut dyn Storage,
) -> StdResult<IntervalEventId> {
    let id: IntervalEventId = INTERVAL_EVENT_ID_COUNTER
        .may_load(store)?
        .unwrap_or_default()
        + 1;
    INTERVAL_EVENT_ID_COUNTER.save(store, &id)?;
    Ok(id)
}

pub(crate) fn push_new_epoch_event(
    storage: &mut dyn Storage,
    env: &Env,
    event: PendingEpochEventKind,
) -> StdResult<EpochEventId> {
    // not included in non-test code as it messes with our return types as we expected `StdResult`
    // from all storage-related operations.
    // However, the callers MUST HAVE ensured the below invariant
    #[cfg(test)]
    crate::support::helpers::ensure_epoch_in_progress_state(storage).unwrap();

    let event_id = next_epoch_event_id_counter(storage)?;
    let event_data = event.attach_source_height(env.block.height);
    PENDING_EPOCH_EVENTS.save(storage, event_id, &event_data)?;
    Ok(event_id)
}

pub(crate) fn push_new_interval_event(
    storage: &mut dyn Storage,
    env: &Env,
    event: PendingIntervalEventKind,
) -> StdResult<IntervalEventId> {
    // not included in non-test code as it messes with our return types as we expected `StdResult`
    // from all storage-related operations.
    // However, the callers MUST HAVE ensured the below invariant
    #[cfg(test)]
    crate::support::helpers::ensure_epoch_in_progress_state(storage).unwrap();

    let event_id = next_interval_event_id_counter(storage)?;
    let event_data = event.attach_source_height(env.block.height);
    PENDING_INTERVAL_EVENTS.save(storage, event_id, &event_data)?;
    Ok(event_id)
}

pub(crate) fn initialise_storage(
    storage: &mut dyn Storage,
    starting_interval: Interval,
    rewarding_validator: Addr,
) -> StdResult<()> {
    save_interval(storage, &starting_interval)?;
    LAST_PROCESSED_EPOCH_EVENT.save(storage, &0)?;
    LAST_PROCESSED_INTERVAL_EVENT.save(storage, &0)?;
    EPOCH_EVENT_ID_COUNTER.save(storage, &0)?;
    INTERVAL_EVENT_ID_COUNTER.save(storage, &0)?;
    CURRENT_EPOCH_STATUS.save(storage, &EpochStatus::new(rewarding_validator))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::fixtures;
    use crate::support::tests::test_helpers::TestSetup;
    use rand_chacha::rand_core::RngCore;

    #[test]
    fn pushing_new_epoch_event_returns_its_id() {
        let mut test = TestSetup::new();
        let env = test.env();

        for _ in 0..500 {
            let dummy_action = PendingEpochEventKind::new_undelegate(
                Addr::unchecked("foomp"),
                test.rng.next_u32(),
            );
            let id = push_new_epoch_event(test.deps_mut().storage, &env, dummy_action).unwrap();
            let expected = EPOCH_EVENT_ID_COUNTER.load(test.deps().storage).unwrap();
            assert_eq!(expected, id);
        }

        test.execute_all_pending_events();

        for _ in 0..10 {
            let dummy_action = PendingEpochEventKind::new_undelegate(
                Addr::unchecked("foomp"),
                test.rng.next_u32(),
            );
            let id = push_new_epoch_event(test.deps_mut().storage, &env, dummy_action).unwrap();
            let expected = EPOCH_EVENT_ID_COUNTER.load(test.deps().storage).unwrap();
            assert_eq!(expected, id);
        }
    }

    #[test]
    fn pushing_new_interval_event_returns_its_id() {
        let mut test = TestSetup::new();
        let env = test.env();

        for _ in 0..500 {
            let dummy_action = PendingIntervalEventKind::ChangeMixCostParams {
                mix_id: test.rng.next_u32(),
                new_costs: fixtures::node_cost_params_fixture(),
            };
            let id = push_new_interval_event(test.deps_mut().storage, &env, dummy_action).unwrap();
            let expected = INTERVAL_EVENT_ID_COUNTER.load(test.deps().storage).unwrap();
            assert_eq!(expected, id);
        }

        test.execute_all_pending_events();

        for _ in 0..10 {
            let dummy_action = PendingIntervalEventKind::ChangeMixCostParams {
                mix_id: test.rng.next_u32(),
                new_costs: fixtures::node_cost_params_fixture(),
            };
            let id = push_new_interval_event(test.deps_mut().storage, &env, dummy_action).unwrap();
            let expected = INTERVAL_EVENT_ID_COUNTER.load(test.deps().storage).unwrap();
            assert_eq!(expected, id);
        }
    }
}

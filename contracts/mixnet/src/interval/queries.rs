// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{
    EPOCH_EVENTS_DEFAULT_RETRIEVAL_LIMIT, EPOCH_EVENTS_MAX_RETRIEVAL_LIMIT,
    INTERVAL_EVENTS_DEFAULT_RETRIEVAL_LIMIT, INTERVAL_EVENTS_MAX_RETRIEVAL_LIMIT,
    REWARDED_SET_DEFAULT_RETRIEVAL_LIMIT, REWARDED_SET_MAX_RETRIEVAL_LIMIT,
};
use crate::interval::storage;
use cosmwasm_std::{Deps, Env, Order, StdResult};
use cw_storage_plus::Bound;
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::pending_events::{PendingEpochEvent, PendingIntervalEvent};
use mixnet_contract_common::{
    CurrentIntervalResponse, EpochEventId, EpochStatus, IntervalEventId, MixId,
    NumberOfPendingEventsResponse, PagedRewardedSetResponse, PendingEpochEventResponse,
    PendingEpochEventsResponse, PendingIntervalEventResponse, PendingIntervalEventsResponse,
};

pub fn query_epoch_status(deps: Deps<'_>) -> StdResult<EpochStatus> {
    storage::current_epoch_status(deps.storage)
}

pub fn query_current_interval_details(
    deps: Deps<'_>,
    env: Env,
) -> StdResult<CurrentIntervalResponse> {
    let interval = storage::current_interval(deps.storage)?;

    Ok(CurrentIntervalResponse::new(interval, env))
}

pub fn query_rewarded_set_paged(
    deps: Deps<'_>,
    start_after: Option<MixId>,
    limit: Option<u32>,
) -> StdResult<PagedRewardedSetResponse> {
    let limit = limit
        .unwrap_or(REWARDED_SET_DEFAULT_RETRIEVAL_LIMIT)
        .min(REWARDED_SET_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let nodes = storage::REWARDED_SET
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = nodes.last().map(|node| node.0);

    Ok(PagedRewardedSetResponse {
        nodes,
        start_next_after,
    })
}

pub fn query_pending_epoch_events_paged(
    deps: Deps<'_>,
    env: Env,
    start_after: Option<EpochEventId>,
    limit: Option<u32>,
) -> StdResult<PendingEpochEventsResponse> {
    let interval = storage::current_interval(deps.storage)?;

    let limit = limit
        .unwrap_or(EPOCH_EVENTS_DEFAULT_RETRIEVAL_LIMIT)
        .min(EPOCH_EVENTS_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let events = storage::PENDING_EPOCH_EVENTS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|row| row.into()))
        .collect::<StdResult<Vec<PendingEpochEvent>>>()?;

    let start_next_after = events.last().map(|event| event.id);

    Ok(PendingEpochEventsResponse {
        seconds_until_executable: interval.secs_until_current_epoch_end(&env),
        events,
        start_next_after,
    })
}

pub fn query_pending_interval_events_paged(
    deps: Deps<'_>,
    env: Env,
    start_after: Option<IntervalEventId>,
    limit: Option<u32>,
) -> StdResult<PendingIntervalEventsResponse> {
    let interval = storage::current_interval(deps.storage)?;

    let limit = limit
        .unwrap_or(INTERVAL_EVENTS_DEFAULT_RETRIEVAL_LIMIT)
        .min(INTERVAL_EVENTS_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let events = storage::PENDING_INTERVAL_EVENTS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|row| row.into()))
        .collect::<StdResult<Vec<PendingIntervalEvent>>>()?;

    let start_next_after = events.last().map(|event| event.id);

    Ok(PendingIntervalEventsResponse {
        seconds_until_executable: interval.secs_until_current_interval_end(&env),
        events,
        start_next_after,
    })
}

pub fn query_pending_epoch_event(
    deps: Deps<'_>,
    event_id: EpochEventId,
) -> Result<PendingEpochEventResponse, MixnetContractError> {
    let event = storage::PENDING_EPOCH_EVENTS.may_load(deps.storage, event_id)?;
    Ok(PendingEpochEventResponse { event_id, event })
}

pub fn query_pending_interval_event(
    deps: Deps<'_>,
    event_id: IntervalEventId,
) -> Result<PendingIntervalEventResponse, MixnetContractError> {
    let event = storage::PENDING_INTERVAL_EVENTS.may_load(deps.storage, event_id)?;
    Ok(PendingIntervalEventResponse { event_id, event })
}

pub fn query_number_of_pending_events(
    deps: Deps<'_>,
) -> Result<NumberOfPendingEventsResponse, MixnetContractError> {
    let last_executed_epoch_id = storage::LAST_PROCESSED_EPOCH_EVENT.load(deps.storage)?;
    let last_inserted_epoch_id = storage::EPOCH_EVENT_ID_COUNTER.load(deps.storage)?;

    let last_executed_interval_id = storage::LAST_PROCESSED_INTERVAL_EVENT.load(deps.storage)?;
    let last_inserted_interval_id = storage::INTERVAL_EVENT_ID_COUNTER.load(deps.storage)?;

    Ok(NumberOfPendingEventsResponse {
        epoch_events: last_inserted_epoch_id - last_executed_epoch_id,
        interval_events: last_inserted_interval_id - last_executed_interval_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::fixtures;
    use crate::support::tests::test_helpers::TestSetup;
    use cosmwasm_std::Addr;
    use mixnet_contract_common::{PendingEpochEventKind, PendingIntervalEventKind};
    use rand_chacha::rand_core::RngCore;

    fn push_n_dummy_epoch_actions(test: &mut TestSetup, n: usize) {
        for _ in 0..n {
            push_dummy_epoch_action(test)
        }
    }

    fn push_dummy_epoch_action(test: &mut TestSetup) {
        let dummy_action =
            PendingEpochEventKind::new_undelegate(Addr::unchecked("foomp"), test.rng.next_u32());
        let env = test.env();
        storage::push_new_epoch_event(test.deps_mut().storage, &env, dummy_action).unwrap();
    }

    fn push_n_dummy_interval_actions(test: &mut TestSetup, n: usize) {
        for _ in 0..n {
            push_dummy_interval_action(test)
        }
    }

    fn push_dummy_interval_action(test: &mut TestSetup) {
        let dummy_action = PendingIntervalEventKind::ChangeMixCostParams {
            mix_id: test.rng.next_u32(),
            new_costs: fixtures::mix_node_cost_params_fixture(),
        };
        let env = test.env();
        storage::push_new_interval_event(test.deps_mut().storage, &env, dummy_action).unwrap();
    }

    #[test]
    fn querying_for_current_interval_details() {
        let mut test = TestSetup::new();

        let interval = test.current_interval();
        let env = test.env();
        let res = query_current_interval_details(test.deps(), env.clone()).unwrap();

        assert_eq!(res.interval, interval);
        assert!(!res.is_current_interval_over);
        assert!(!res.is_current_epoch_over);
        assert_eq!(res.current_blocktime, env.block.time.seconds());

        test.skip_to_current_epoch_end();
        let interval = test.current_interval();
        let env = test.env();
        let res = query_current_interval_details(test.deps(), env.clone()).unwrap();

        assert_eq!(res.interval, interval);
        assert!(!res.is_current_interval_over);
        assert!(res.is_current_epoch_over);
        assert_eq!(res.current_blocktime, env.block.time.seconds());

        test.skip_to_current_interval_end();
        let interval = test.current_interval();
        let env = test.env();
        let res = query_current_interval_details(test.deps(), env.clone()).unwrap();

        assert_eq!(res.interval, interval);
        assert!(res.is_current_interval_over);
        assert!(res.is_current_epoch_over);
        assert_eq!(res.current_blocktime, env.block.time.seconds());
    }

    #[cfg(test)]
    mod rewarded_set {
        use super::*;

        fn set_rewarded_set_to_n_nodes(test: &mut TestSetup, n: usize) {
            let set = (1u32..).take(n).collect::<Vec<_>>();
            test.force_change_rewarded_set(set)
        }

        #[test]
        fn obeys_limits() {
            let mut test = TestSetup::new();
            set_rewarded_set_to_n_nodes(&mut test, 200);

            let limit = 2;
            let page1 = query_rewarded_set_paged(test.deps(), None, Some(limit)).unwrap();
            assert_eq!(limit, page1.nodes.len() as u32);
        }

        #[test]
        fn has_default_limit() {
            let mut test = TestSetup::new();
            set_rewarded_set_to_n_nodes(&mut test, 2000);

            // query without explicitly setting a limit
            let page1 = query_rewarded_set_paged(test.deps(), None, None).unwrap();

            assert_eq!(
                REWARDED_SET_DEFAULT_RETRIEVAL_LIMIT,
                page1.nodes.len() as u32
            );
        }

        #[test]
        fn has_max_limit() {
            let mut test = TestSetup::new();
            set_rewarded_set_to_n_nodes(&mut test, 2000);

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 10000;
            let page1 = query_rewarded_set_paged(test.deps(), None, Some(crazy_limit)).unwrap();

            assert_eq!(REWARDED_SET_MAX_RETRIEVAL_LIMIT, page1.nodes.len() as u32);
        }

        #[test]
        fn pagination_works() {
            let mut test = TestSetup::new();

            set_rewarded_set_to_n_nodes(&mut test, 1);

            let per_page = 2;
            let page1 = query_rewarded_set_paged(test.deps(), None, Some(per_page)).unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.nodes.len());

            set_rewarded_set_to_n_nodes(&mut test, 2);

            // page1 should have 2 results on it
            let page1 = query_rewarded_set_paged(test.deps(), None, Some(per_page)).unwrap();
            assert_eq!(2, page1.nodes.len());

            set_rewarded_set_to_n_nodes(&mut test, 3);

            // page1 still has the same 2 results
            let another_page1 =
                query_rewarded_set_paged(test.deps(), None, Some(per_page)).unwrap();
            assert_eq!(2, another_page1.nodes.len());
            assert_eq!(page1, another_page1);

            // retrieving the next page should start after the last key on this page
            let start_after = page1.start_next_after.unwrap();
            let page2 =
                query_rewarded_set_paged(test.deps(), Some(start_after), Some(per_page)).unwrap();

            assert_eq!(1, page2.nodes.len());

            // save another one
            set_rewarded_set_to_n_nodes(&mut test, 4);

            let page2 =
                query_rewarded_set_paged(test.deps(), Some(start_after), Some(per_page)).unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.nodes.len());
        }
    }

    #[cfg(test)]
    mod pending_epoch_events {
        use super::*;

        #[test]
        fn obeys_limits() {
            let mut test = TestSetup::new();
            push_n_dummy_epoch_actions(&mut test, 100);
            let env = test.env();

            let limit = 2;

            let page1 =
                query_pending_epoch_events_paged(test.deps(), env, None, Some(limit)).unwrap();
            assert_eq!(limit, page1.events.len() as u32);
        }

        #[test]
        fn has_default_limit() {
            let mut test = TestSetup::new();
            push_n_dummy_epoch_actions(&mut test, 1000);
            let env = test.env();

            // query without explicitly setting a limit
            let page1 = query_pending_epoch_events_paged(test.deps(), env, None, None).unwrap();

            assert_eq!(
                EPOCH_EVENTS_DEFAULT_RETRIEVAL_LIMIT,
                page1.events.len() as u32
            );
        }

        #[test]
        fn has_max_limit() {
            let mut test = TestSetup::new();
            push_n_dummy_epoch_actions(&mut test, 1000);
            let env = test.env();

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 10000;
            let page1 = query_pending_epoch_events_paged(test.deps(), env, None, Some(crazy_limit))
                .unwrap();

            assert_eq!(EPOCH_EVENTS_MAX_RETRIEVAL_LIMIT, page1.events.len() as u32);
        }

        #[test]
        fn pagination_works() {
            let mut test = TestSetup::new();
            let env = test.env();
            push_dummy_epoch_action(&mut test);

            let per_page = 2;
            let page1 =
                query_pending_epoch_events_paged(test.deps(), env.clone(), None, Some(per_page))
                    .unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.events.len());

            // save another
            push_dummy_epoch_action(&mut test);

            // page1 should have 2 results on it
            let page1 =
                query_pending_epoch_events_paged(test.deps(), env.clone(), None, Some(per_page))
                    .unwrap();
            assert_eq!(2, page1.events.len());

            push_dummy_epoch_action(&mut test);

            // page1 still has the same 2 results
            let another_page1 =
                query_pending_epoch_events_paged(test.deps(), env.clone(), None, Some(per_page))
                    .unwrap();
            assert_eq!(2, another_page1.events.len());
            assert_eq!(page1, another_page1);

            // retrieving the next page should start after the last key on this page
            let start_after = page1.start_next_after.unwrap();
            let page2 = query_pending_epoch_events_paged(
                test.deps(),
                env.clone(),
                Some(start_after),
                Some(per_page),
            )
            .unwrap();

            assert_eq!(1, page2.events.len());

            // save another one
            push_dummy_epoch_action(&mut test);

            let page2 = query_pending_epoch_events_paged(
                test.deps(),
                env,
                Some(start_after),
                Some(per_page),
            )
            .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.events.len());
        }

        #[test]
        fn shows_correct_time_until_possible_execution() {
            let mut test = TestSetup::new();
            let env = test.env();
            push_dummy_epoch_action(&mut test);

            let res =
                query_pending_epoch_events_paged(test.deps(), env.clone(), None, None).unwrap();
            let interval = test.current_interval();

            // it's essentially always the time until the epoch end
            assert_eq!(
                res.seconds_until_executable,
                interval.secs_until_current_epoch_end(&env)
            )
        }
    }

    #[cfg(test)]
    mod pending_interval_events {
        use super::*;

        #[test]
        fn obeys_limits() {
            let mut test = TestSetup::new();
            push_n_dummy_interval_actions(&mut test, 100);
            let env = test.env();

            let limit = 2;

            let page1 =
                query_pending_interval_events_paged(test.deps(), env, None, Some(limit)).unwrap();
            assert_eq!(limit, page1.events.len() as u32);
        }

        #[test]
        fn has_default_limit() {
            let mut test = TestSetup::new();
            push_n_dummy_interval_actions(&mut test, 1000);
            let env = test.env();

            // query without explicitly setting a limit
            let page1 = query_pending_interval_events_paged(test.deps(), env, None, None).unwrap();

            assert_eq!(
                INTERVAL_EVENTS_DEFAULT_RETRIEVAL_LIMIT,
                page1.events.len() as u32
            );
        }

        #[test]
        fn has_max_limit() {
            let mut test = TestSetup::new();
            push_n_dummy_interval_actions(&mut test, 1000);
            let env = test.env();

            // query with a crazily high limit in an attempt to use too many resources
            let crazy_limit = 10000;
            let page1 =
                query_pending_interval_events_paged(test.deps(), env, None, Some(crazy_limit))
                    .unwrap();

            assert_eq!(
                INTERVAL_EVENTS_MAX_RETRIEVAL_LIMIT,
                page1.events.len() as u32
            );
        }

        #[test]
        fn pagination_works() {
            let mut test = TestSetup::new();
            let env = test.env();
            push_dummy_interval_action(&mut test);

            let per_page = 2;
            let page1 =
                query_pending_interval_events_paged(test.deps(), env.clone(), None, Some(per_page))
                    .unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.events.len());

            // save another
            push_dummy_interval_action(&mut test);

            // page1 should have 2 results on it
            let page1 =
                query_pending_interval_events_paged(test.deps(), env.clone(), None, Some(per_page))
                    .unwrap();
            assert_eq!(2, page1.events.len());

            push_dummy_interval_action(&mut test);

            // page1 still has the same 2 results
            let another_page1 =
                query_pending_interval_events_paged(test.deps(), env.clone(), None, Some(per_page))
                    .unwrap();
            assert_eq!(2, another_page1.events.len());
            assert_eq!(page1, another_page1);

            // retrieving the next page should start after the last key on this page
            let start_after = page1.start_next_after.unwrap();
            let page2 = query_pending_interval_events_paged(
                test.deps(),
                env.clone(),
                Some(start_after),
                Some(per_page),
            )
            .unwrap();

            assert_eq!(1, page2.events.len());

            // save another one
            push_dummy_interval_action(&mut test);

            let page2 = query_pending_interval_events_paged(
                test.deps(),
                env,
                Some(start_after),
                Some(per_page),
            )
            .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.events.len());
        }

        #[test]
        fn shows_correct_time_until_possible_execution() {
            let mut test = TestSetup::new();
            let env = test.env();
            push_dummy_interval_action(&mut test);

            let res =
                query_pending_interval_events_paged(test.deps(), env.clone(), None, None).unwrap();
            let interval = test.current_interval();

            // it's essentially always the time until the interval end
            assert_eq!(
                res.seconds_until_executable,
                interval.secs_until_current_interval_end(&env)
            )
        }
    }

    #[test]
    fn query_for_pending_epoch_event() {
        let mut test = TestSetup::new();

        // it doesn't exist
        let expected = PendingEpochEventResponse {
            event_id: 123,
            event: None,
        };
        assert_eq!(
            expected,
            query_pending_epoch_event(test.deps(), 123).unwrap()
        );

        // it exists
        let dummy_action =
            PendingEpochEventKind::new_undelegate(Addr::unchecked("foomp"), test.rng.next_u32());
        let env = test.env();
        storage::push_new_epoch_event(test.deps_mut().storage, &env, dummy_action.clone()).unwrap();
        let expected = PendingEpochEventResponse {
            event_id: 1,
            event: Some(dummy_action.attach_source_height(env.block.height)),
        };

        assert_eq!(expected, query_pending_epoch_event(test.deps(), 1).unwrap());

        // it no longer exist (but used to)
        test.execute_all_pending_events();
        let expected = PendingEpochEventResponse {
            event_id: 1,
            event: None,
        };
        assert_eq!(expected, query_pending_epoch_event(test.deps(), 1).unwrap());
    }

    #[test]
    fn query_for_pending_interval_event() {
        let mut test = TestSetup::new();

        // it doesn't exist
        let expected = PendingIntervalEventResponse {
            event_id: 123,
            event: None,
        };
        assert_eq!(
            expected,
            query_pending_interval_event(test.deps(), 123).unwrap()
        );

        // it exists
        let dummy_action = PendingIntervalEventKind::ChangeMixCostParams {
            mix_id: test.rng.next_u32(),
            new_costs: fixtures::mix_node_cost_params_fixture(),
        };
        let env = test.env();
        storage::push_new_interval_event(test.deps_mut().storage, &env, dummy_action.clone())
            .unwrap();
        let expected = PendingIntervalEventResponse {
            event_id: 1,
            event: Some(dummy_action.attach_source_height(env.block.height)),
        };

        assert_eq!(
            expected,
            query_pending_interval_event(test.deps(), 1).unwrap()
        );

        // it no longer exist (but used to)
        test.execute_all_pending_events();
        let expected = PendingIntervalEventResponse {
            event_id: 1,
            event: None,
        };
        assert_eq!(
            expected,
            query_pending_interval_event(test.deps(), 1).unwrap()
        );
    }

    #[test]
    fn querying_for_number_of_pending_events() {
        let mut test = TestSetup::new();

        // no events
        assert_eq!(
            Ok(NumberOfPendingEventsResponse {
                epoch_events: 0,
                interval_events: 0
            }),
            query_number_of_pending_events(test.deps())
        );

        // add epoch event
        push_dummy_epoch_action(&mut test);
        assert_eq!(
            Ok(NumberOfPendingEventsResponse {
                epoch_events: 1,
                interval_events: 0
            }),
            query_number_of_pending_events(test.deps())
        );

        // add more epoch events
        push_n_dummy_epoch_actions(&mut test, 42);
        assert_eq!(
            Ok(NumberOfPendingEventsResponse {
                epoch_events: 43,
                interval_events: 0
            }),
            query_number_of_pending_events(test.deps())
        );

        // and now for interval...
        // add interval event
        push_dummy_interval_action(&mut test);
        assert_eq!(
            Ok(NumberOfPendingEventsResponse {
                epoch_events: 43,
                interval_events: 1
            }),
            query_number_of_pending_events(test.deps())
        );

        // add more epoch events
        push_n_dummy_interval_actions(&mut test, 42);
        assert_eq!(
            Ok(NumberOfPendingEventsResponse {
                epoch_events: 43,
                interval_events: 43
            }),
            query_number_of_pending_events(test.deps())
        );
    }
}

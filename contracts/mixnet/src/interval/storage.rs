// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{
    CURRENT_INTERVAL_KEY, EPOCH_EVENT_ID_COUNTER_KEY, INTERVAL_EVENT_ID_COUNTER_KEY,
    LAST_EPOCH_EVENT_ID_KEY, LAST_INTERVAL_EVENT_ID_KEY, PENDING_EPOCH_EVENTS_NAMESPACE,
    PENDING_INTERVAL_EVENTS_NAMESPACE, REWARDED_SET_KEY,
};
use cosmwasm_std::{Order, StdResult, Storage};
use cw_storage_plus::{Item, Map};
use mixnet_contract_common::pending_events::{PendingEpochEvent, PendingIntervalEvent};
use mixnet_contract_common::{Interval, NodeId, RewardedSetNodeStatus};
use std::collections::HashMap;

pub(crate) type EventId = u32;

pub(crate) const CURRENT_INTERVAL: Item<'_, Interval> = Item::new(CURRENT_INTERVAL_KEY);
pub(crate) const REWARDED_SET: Map<NodeId, RewardedSetNodeStatus> = Map::new(REWARDED_SET_KEY);

pub(crate) const EPOCH_EVENT_ID_COUNTER: Item<EventId> = Item::new(EPOCH_EVENT_ID_COUNTER_KEY);
pub(crate) const INTERVAL_EVENT_ID_COUNTER: Item<EventId> =
    Item::new(INTERVAL_EVENT_ID_COUNTER_KEY);

pub(crate) const LAST_PROCESSED_EPOCH_EVENT: Item<EventId> = Item::new(LAST_EPOCH_EVENT_ID_KEY);
pub(crate) const LAST_PROCESSED_INTERVAL_EVENT: Item<EventId> =
    Item::new(LAST_INTERVAL_EVENT_ID_KEY);

// we're indexing the events by an increasing ID so that we'd execute them in the order they were created
// (we can't use block height as it's very possible multiple requests might be created in the same block height,
// and composite keys would be more complex than just using an increasing ID)
/// Contains operations that should get resolved at the end of the current epoch.
pub(crate) const PENDING_EPOCH_EVENTS: Map<EventId, PendingEpochEvent> =
    Map::new(PENDING_EPOCH_EVENTS_NAMESPACE);

/// Contains operations that should get resolved at the end of the current interval.
pub(crate) const PENDING_INTERVAL_EVENTS: Map<EventId, PendingIntervalEvent> =
    Map::new(PENDING_INTERVAL_EVENTS_NAMESPACE);

pub(crate) fn current_interval(storage: &dyn Storage) -> StdResult<Interval> {
    CURRENT_INTERVAL.load(storage)
}

pub(crate) fn save_interval(storage: &mut dyn Storage, interval: &Interval) -> StdResult<()> {
    CURRENT_INTERVAL.save(storage, interval)
}

pub(crate) fn next_epoch_event_id_counter(store: &mut dyn Storage) -> StdResult<EventId> {
    let id: EventId = EPOCH_EVENT_ID_COUNTER.may_load(store)?.unwrap_or_default() + 1;
    EPOCH_EVENT_ID_COUNTER.save(store, &id)?;
    Ok(id)
}

pub(crate) fn next_interval_event_id_counter(store: &mut dyn Storage) -> StdResult<EventId> {
    let id: EventId = INTERVAL_EVENT_ID_COUNTER
        .may_load(store)?
        .unwrap_or_default()
        + 1;
    INTERVAL_EVENT_ID_COUNTER.save(store, &id)?;
    Ok(id)
}

pub(crate) fn push_new_epoch_event(
    storage: &mut dyn Storage,
    event: &PendingEpochEvent,
) -> StdResult<()> {
    let event_id = next_epoch_event_id_counter(storage)?;
    PENDING_EPOCH_EVENTS.save(storage, event_id, event)
}

pub(crate) fn push_new_interval_event(
    storage: &mut dyn Storage,
    event: &PendingIntervalEvent,
) -> StdResult<()> {
    let event_id = next_interval_event_id_counter(storage)?;
    PENDING_INTERVAL_EVENTS.save(storage, event_id, event)
}

// pub(crate) fn save_rewarded_set(
//     storage: &mut dyn Storage,
//     active_set_size: u32,
//     entries: Vec<NodeId>,
// ) -> StdResult<()> {
//     for (i, node_id) in entries.into_iter().enumerate() {
//         // first k nodes are active
//         let set_status = if i < active_set_size as usize {
//             RewardedSetNodeStatus::Active
//         } else {
//             RewardedSetNodeStatus::Standby
//         };
//
//         REWARDED_SET.save(storage, node_id, &set_status)?;
//     }
//
//     Ok(())
// }

pub(crate) fn update_rewarded_set(
    storage: &mut dyn Storage,
    active_set_size: u32,
    new_set: Vec<NodeId>,
) -> StdResult<()> {
    // our goal is to reduce the number of reads and writes to the underlying storage,
    // whilst completely overwriting the current rewarded set.
    // the naive implementation would be to read the entire current rewarded set,
    // remove all of those entries
    // and write the new one in its place.
    // However, very often it might turn out that a node hasn't changed its status in the updated epoch,
    // and in those cases we can save on having to remove the entry and writing a new one.

    // Note: so far it seems the contract compiles (and stores) fine with a `HashMap`, but if we ever
    // run into any issues due to any randomness? we can switch it up for a BTreeMap
    let mut old_nodes = REWARDED_SET
        .range(storage, None, None, Order::Ascending)
        .collect::<Result<HashMap<_, _>, _>>()?;

    for (i, node_id) in new_set.into_iter().enumerate() {
        // first k nodes are active
        let set_status = if i < active_set_size as usize {
            RewardedSetNodeStatus::Active
        } else {
            RewardedSetNodeStatus::Standby
        };

        if !matches!(old_nodes.get(&node_id), Some(status) if status == &set_status) {
            // if the status changed, or didn't exist, write it down:
            REWARDED_SET.save(storage, node_id, &set_status)?;
        }

        old_nodes.remove(&node_id);
    }

    // finally remove the entries for nodes that no longer exist [in the rewarded set]
    for old_node_id in old_nodes.keys() {
        REWARDED_SET.remove(storage, *old_node_id)
    }

    Ok(())
}

pub(crate) fn initialise_storage(
    storage: &mut dyn Storage,
    starting_interval: Interval,
) -> StdResult<()> {
    save_interval(storage, &starting_interval)?;
    LAST_PROCESSED_EPOCH_EVENT.save(storage, &0)?;
    LAST_PROCESSED_INTERVAL_EVENT.save(storage, &0)?;
    EPOCH_EVENT_ID_COUNTER.save(storage, &0)?;
    INTERVAL_EVENT_ID_COUNTER.save(storage, &0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::Order;

    fn read_entire_set(storage: &mut dyn Storage) -> HashMap<NodeId, RewardedSetNodeStatus> {
        REWARDED_SET
            .range(storage, None, None, Order::Ascending)
            .map(|r| r.unwrap())
            .collect()
    }

    #[test]
    fn updating_rewarded_set() {
        // just some variables to keep test assertions more concise
        let active = &RewardedSetNodeStatus::Active;
        let standby = &RewardedSetNodeStatus::Standby;

        let mut deps = mock_dependencies();
        let store = deps.as_mut().storage;
        assert!(read_entire_set(store).is_empty());

        // writing initial rewarded set shouldn't do anything fancy
        update_rewarded_set(store, 2, vec![6, 2, 7, 4, 1]).unwrap();
        let current_set = read_entire_set(store);
        assert_eq!(current_set.len(), 5);
        assert_eq!(active, current_set.get(&6).unwrap());
        assert_eq!(active, current_set.get(&2).unwrap());
        assert_eq!(standby, current_set.get(&7).unwrap());
        assert_eq!(standby, current_set.get(&4).unwrap());
        assert_eq!(standby, current_set.get(&1).unwrap());
        assert!(current_set.get(&42).is_none());

        update_rewarded_set(store, 2, vec![2, 5, 6, 3, 4]).unwrap();
        let current_set = read_entire_set(store);
        assert_eq!(current_set.len(), 5);
        assert_eq!(active, current_set.get(&2).unwrap());
        assert_eq!(active, current_set.get(&5).unwrap());
        assert_eq!(standby, current_set.get(&6).unwrap());
        assert_eq!(standby, current_set.get(&3).unwrap());
        assert_eq!(standby, current_set.get(&4).unwrap());
        // those no longer are in the rewarded set
        assert!(current_set.get(&7).is_none());
        assert!(current_set.get(&1).is_none());
    }
}

//
// // type aliases for better reasoning for storage keys
// // (I found it helpful)
// type BlockHeight = u64;
// type IntervalId = u32;
//
// // TODO: those values need to be verified
// pub(crate) const REWARDED_NODE_DEFAULT_PAGE_LIMIT: u32 = 1000;
// pub(crate) const REWARDED_NODE_MAX_PAGE_LIMIT: u32 = 1500;
//
// pub(crate) const CURRENT_EPOCH: Item<'_, Interval> = Item::new("ceph");
// pub(crate) const CURRENT_EPOCH_REWARD_PARAMS: Item<'_, EpochRewardParams> = Item::new("erp");
// pub(crate) const CURRENT_REWARDED_SET_HEIGHT: Item<'_, BlockHeight> = Item::new("crh");
//
// // I've changed the `()` data to an `u8` as after serializing `()` is represented as "null",
// // taking more space than a single digit u8. If we don't care about what's there, why not go with more efficient approach? : )
// // pub(crate) const REWARDED_SET_HEIGHTS_FOR_INTERVAL: Map<'_, (IntervalId, BlockHeight), u8> =
// //     Map::new("rsh");
//
// // pub(crate) const REWARDED_SET: Map<(u64, IdentityKey), NodeStatus> = Map::new("rs");
// pub(crate) const REWARDED_SET: Map<'_, (BlockHeight, IdentityKey), RewardedSetNodeStatus> =
//     Map::new("rs");
//
// pub(crate) const EPOCHS: Map<'_, IntervalId, Interval> = Map::new("ephs");
//
// pub fn save_epoch(storage: &mut dyn Storage, epoch: &Interval) -> Result<(), ContractError> {
//     CURRENT_EPOCH.save(storage, epoch)?;
//     EPOCHS.save(storage, epoch.id(), epoch)?;
//     Ok(())
// }
//
// pub fn current_epoch_reward_params(
//     storage: &dyn Storage,
// ) -> Result<EpochRewardParams, ContractError> {
//     Ok(CURRENT_EPOCH_REWARD_PARAMS.load(storage)?)
// }
//
// pub fn save_epoch_reward_params(
//     epoch_id: u32,
//     storage: &mut dyn Storage,
// ) -> Result<(), ContractError> {
//     let epoch_reward_params = epoch_reward_params(storage)?;
//     CURRENT_EPOCH_REWARD_PARAMS.save(storage, &epoch_reward_params)?;
//     crate::rewards::storage::EPOCH_REWARD_PARAMS.save(storage, epoch_id, &epoch_reward_params)?;
//     Ok(())
// }
//
// pub fn current_epoch(storage: &dyn Storage) -> Result<Interval, ContractError> {
//     CURRENT_EPOCH
//         .load(storage)
//         .map_err(|_| ContractError::EpochNotInitialized)
// }
//
// pub(crate) fn save_rewarded_set(
//     storage: &mut dyn Storage,
//     height: BlockHeight,
//     active_set_size: u32,
//     entries: Vec<IdentityKey>,
// ) -> StdResult<()> {
//     for (i, identity) in entries.into_iter().enumerate() {
//         // first k nodes are active
//         let set_status = if i < active_set_size as usize {
//             RewardedSetNodeStatus::Active
//         } else {
//             RewardedSetNodeStatus::Standby
//         };
//
//         REWARDED_SET.save(storage, (height, identity), &set_status)?;
//     }
//
//     Ok(())
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::support::tests::test_helpers;
//
//     #[test]
//     fn saving_rewarded_set() {
//         let mut deps = test_helpers::init_contract();
//
//         let active_set_size = 100;
//         let mut nodes = Vec::new();
//         for i in 0..1000 {
//             nodes.push(format!("identity{:04}", i))
//         }
//
//         save_rewarded_set(deps.as_mut().storage, 1234, active_set_size, nodes).unwrap();
//
//         // first k nodes MUST BE active
//         for i in 0..1000 {
//             let identity = format!("identity{:04}", i);
//             if i < active_set_size {
//                 assert_eq!(
//                     RewardedSetNodeStatus::Active,
//                     REWARDED_SET
//                         .load(deps.as_ref().storage, (1234, identity))
//                         .unwrap()
//                 )
//             } else {
//                 assert_eq!(
//                     RewardedSetNodeStatus::Standby,
//                     REWARDED_SET
//                         .load(deps.as_ref().storage, (1234, identity))
//                         .unwrap()
//                 )
//             }
//         }
//     }
// }

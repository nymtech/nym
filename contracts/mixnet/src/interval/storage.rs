// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Order, StdResult, Storage};
use cw_storage_plus::{Item, Map};
use mixnet_contract_common::error::MixnetContractError;
use mixnet_contract_common::{Interval, NodeId, RewardedSetNodeStatus};
use std::collections::HashMap;

const REWARDED_SET_KEY: &str = "rs";
const CURRENT_INTERVAL_KEY: &str = "ci";

pub(crate) const CURRENT_INTERVAL: Item<'_, Interval> = Item::new(CURRENT_INTERVAL_KEY);
pub(crate) const REWARDED_SET: Map<NodeId, RewardedSetNodeStatus> = Map::new(REWARDED_SET_KEY);

pub(crate) fn current_interval(storage: &dyn Storage) -> Result<Interval, MixnetContractError> {
    Ok(CURRENT_INTERVAL.load(storage)?)
}

pub(crate) fn save_interval(
    storage: &mut dyn Storage,
    interval: &Interval,
) -> Result<(), MixnetContractError> {
    Ok(CURRENT_INTERVAL.save(storage, interval)?)
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
    // TODO: read here the size of our current rewarded set

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

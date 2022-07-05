// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// use super::storage;
// use crate::error::ContractError;
// use cosmwasm_std::{Env, Order, StdResult, Storage};
// use cw_storage_plus::Bound;
// use mixnet_contract_common::{
//     IdentityKey, Interval, PagedRewardedSetResponse, RewardedSetNodeStatus,
//     RewardedSetUpdateDetails,
// };
//
// pub fn query_current_epoch(storage: &dyn Storage) -> Result<Interval, ContractError> {
//     storage::current_epoch(storage)
// }
//
// pub(crate) fn query_rewarded_set_refresh_minimum_blocks() -> u64 {
//     crate::constants::REWARDED_SET_REFRESH_BLOCKS
// }
//
// // note: I have removed the `query_rewarded_set_for_interval`, because I don't think it's appropriate
// // for the contract to go through so much data (i.e. all "rewarded" sets of particular interval) in one go.
// // To achieve the same result, the client would have to instead first call `query_rewarded_set_heights_for_interval`
// // to learn the heights used in given interval and then for each of them `query_rewarded_set` for that particular height.
//
// pub fn query_current_rewarded_set_height(storage: &dyn Storage) -> Result<u64, ContractError> {
//     Ok(storage::CURRENT_REWARDED_SET_HEIGHT.load(storage)?)
// }
//
// fn query_rewarded_set_at_height(
//     storage: &dyn Storage,
//     height: u64,
//     start_after: Option<IdentityKey>,
//     limit: u32,
// ) -> Result<Vec<(IdentityKey, RewardedSetNodeStatus)>, ContractError> {
//     let start = start_after.map(Bound::exclusive);
//
//     let rewarded_set = storage::REWARDED_SET
//         .prefix(height)
//         .range(storage, start, None, Order::Ascending)
//         .take(limit as usize)
//         .collect::<StdResult<_>>()?;
//     Ok(rewarded_set)
// }
//
// pub fn query_rewarded_set(
//     storage: &dyn Storage,
//     height: Option<u64>,
//     start_after: Option<IdentityKey>,
//     limit: Option<u32>,
// ) -> Result<PagedRewardedSetResponse, ContractError> {
//     let height = match height {
//         Some(height) => height,
//         None => query_current_rewarded_set_height(storage)?,
//     };
//     let limit = limit
//         .unwrap_or(storage::REWARDED_NODE_DEFAULT_PAGE_LIMIT)
//         .min(storage::REWARDED_NODE_MAX_PAGE_LIMIT);
//
//     // query for an additional element to determine paging requirements
//     let mut paged_result = query_rewarded_set_at_height(storage, height, start_after, limit + 1)?;
//
//     if paged_result.len() > limit as usize {
//         paged_result.truncate(limit as usize);
//         Ok(PagedRewardedSetResponse {
//             start_next_after: paged_result.last().map(|res| res.0.clone()),
//             identities: paged_result,
//             at_height: height,
//         })
//     } else {
//         Ok(PagedRewardedSetResponse {
//             identities: paged_result,
//             start_next_after: None,
//             at_height: height,
//         })
//     }
// }
//
// // this was all put together into the same query so that all information would be synced together
// pub fn query_rewarded_set_update_details(
//     env: Env,
//     storage: &dyn Storage,
// ) -> Result<RewardedSetUpdateDetails, ContractError> {
//     Ok(RewardedSetUpdateDetails {
//         refresh_rate_blocks: query_rewarded_set_refresh_minimum_blocks(),
//         last_refreshed_block: query_current_rewarded_set_height(storage)?,
//         current_height: env.block.height,
//     })
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::interval::storage::REWARDED_NODE_MAX_PAGE_LIMIT;
//     use crate::support::tests::test_helpers;
//     use cosmwasm_std::testing::mock_env;
//
//     fn store_rewarded_nodes(
//         storage: &mut dyn Storage,
//         height: u64,
//         active_set: u32,
//         rewarded_set: u32,
//     ) -> Vec<IdentityKey> {
//         let identities = (0..rewarded_set)
//             .map(|i| format!("identity{:04}", i))
//             .collect::<Vec<_>>();
//         storage::save_rewarded_set(storage, height, active_set, identities.clone()).unwrap();
//         identities
//     }
//
//     #[test]
//     fn querying_for_rewarded_set_at_height() {
//         let mut deps = test_helpers::init_contract();
//
//         // store some nodes
//         let identities1 = store_rewarded_nodes(deps.as_mut().storage, 1, 100, 200);
//         let identities2 = store_rewarded_nodes(deps.as_mut().storage, 2, 50, 200);
//         let identities3 = store_rewarded_nodes(deps.as_mut().storage, 3, 150, 200);
//         let identities4 = store_rewarded_nodes(deps.as_mut().storage, 4, 300, 500);
//         let identities5 = store_rewarded_nodes(deps.as_mut().storage, 5, 500, 500);
//
//         // expected2 and 3 are basically sanity checks to ensure changing active set size (increase or decrease)
//         // doesn't affect the ordering
//
//         let expected1 = identities1
//             .into_iter()
//             .enumerate()
//             .map(|(i, identity)| {
//                 if i < 100 {
//                     (identity, RewardedSetNodeStatus::Active)
//                 } else {
//                     (identity, RewardedSetNodeStatus::Standby)
//                 }
//             })
//             .collect::<Vec<_>>();
//
//         assert_eq!(
//             expected1,
//             query_rewarded_set_at_height(deps.as_ref().storage, 1, None, 1000).unwrap()
//         );
//
//         let expected2 = identities2
//             .into_iter()
//             .enumerate()
//             .map(|(i, identity)| {
//                 if i < 50 {
//                     (identity, RewardedSetNodeStatus::Active)
//                 } else {
//                     (identity, RewardedSetNodeStatus::Standby)
//                 }
//             })
//             .collect::<Vec<_>>();
//
//         assert_eq!(
//             expected2,
//             query_rewarded_set_at_height(deps.as_ref().storage, 2, None, 1000).unwrap()
//         );
//
//         let expected3 = identities3
//             .into_iter()
//             .enumerate()
//             .map(|(i, identity)| {
//                 if i < 150 {
//                     (identity, RewardedSetNodeStatus::Active)
//                 } else {
//                     (identity, RewardedSetNodeStatus::Standby)
//                 }
//             })
//             .collect::<Vec<_>>();
//
//         assert_eq!(
//             expected3,
//             query_rewarded_set_at_height(deps.as_ref().storage, 3, None, 1000).unwrap()
//         );
//
//         // check limit and paging
//         // active: 300, rewarded: 500
//         let first_100 = identities4
//             .iter()
//             .take(100)
//             .map(|identity| (identity.clone(), RewardedSetNodeStatus::Active))
//             .collect::<Vec<_>>();
//         assert_eq!(
//             first_100,
//             query_rewarded_set_at_height(deps.as_ref().storage, 4, None, 100).unwrap()
//         );
//
//         let expected_single1 = vec![("identity0299".to_string(), RewardedSetNodeStatus::Active)];
//         let expected_single2 = vec![("identity0300".to_string(), RewardedSetNodeStatus::Standby)];
//         assert_eq!(
//             expected_single1,
//             query_rewarded_set_at_height(
//                 deps.as_ref().storage,
//                 4,
//                 Some("identity0298".to_string()),
//                 1
//             )
//             .unwrap()
//         );
//         assert_eq!(
//             expected_single2,
//             query_rewarded_set_at_height(
//                 deps.as_ref().storage,
//                 4,
//                 Some("identity0299".to_string()),
//                 1
//             )
//             .unwrap()
//         );
//
//         let last_100 = identities4
//             .iter()
//             .skip(400)
//             .map(|identity| (identity.clone(), RewardedSetNodeStatus::Standby))
//             .collect::<Vec<_>>();
//         assert_eq!(
//             last_100,
//             query_rewarded_set_at_height(
//                 deps.as_ref().storage,
//                 4,
//                 Some("identity0399".to_string()),
//                 100
//             )
//             .unwrap()
//         );
//
//         // all nodes are in the active set
//         let expected5 = identities5
//             .into_iter()
//             .map(|identity| (identity, RewardedSetNodeStatus::Active))
//             .collect::<Vec<_>>();
//
//         assert_eq!(
//             expected5,
//             query_rewarded_set_at_height(deps.as_ref().storage, 5, None, 1000).unwrap()
//         );
//     }
//
//     #[test]
//     fn querying_for_rewarded_set() {
//         let mut deps = test_helpers::init_contract();
//
//         let current_height = 123;
//         let other_height = 456;
//         let different_height = 789;
//
//         storage::CURRENT_REWARDED_SET_HEIGHT
//             .save(deps.as_mut().storage, &current_height)
//             .unwrap();
//
//         let identities1 = store_rewarded_nodes(deps.as_mut().storage, current_height, 50, 100);
//         let identities2 = store_rewarded_nodes(deps.as_mut().storage, other_height, 100, 200);
//         let identities3 = store_rewarded_nodes(
//             deps.as_mut().storage,
//             different_height,
//             storage::REWARDED_NODE_MAX_PAGE_LIMIT,
//             storage::REWARDED_NODE_MAX_PAGE_LIMIT * 2,
//         );
//
//         // if height is not set, current height is used, else it's just passed
//         let expected1 = PagedRewardedSetResponse {
//             identities: identities1
//                 .into_iter()
//                 .enumerate()
//                 .map(|(i, identity)| {
//                     if i < 50 {
//                         (identity, RewardedSetNodeStatus::Active)
//                     } else {
//                         (identity, RewardedSetNodeStatus::Standby)
//                     }
//                 })
//                 .collect::<Vec<_>>(),
//             start_next_after: None,
//             at_height: current_height,
//         };
//         let expected2 = PagedRewardedSetResponse {
//             identities: identities2
//                 .into_iter()
//                 .enumerate()
//                 .map(|(i, identity)| {
//                     if i < 100 {
//                         (identity, RewardedSetNodeStatus::Active)
//                     } else {
//                         (identity, RewardedSetNodeStatus::Standby)
//                     }
//                 })
//                 .collect::<Vec<_>>(),
//             start_next_after: None,
//             at_height: other_height,
//         };
//
//         assert_eq!(
//             Ok(expected1),
//             query_rewarded_set(deps.as_ref().storage, None, None, None)
//         );
//         assert_eq!(
//             Ok(expected2),
//             query_rewarded_set(deps.as_ref().storage, Some(other_height), None, None)
//         );
//
//         // if limit is not set, a default one is used instead
//         let expected3 = PagedRewardedSetResponse {
//             identities: identities3
//                 .iter()
//                 .take(storage::REWARDED_NODE_DEFAULT_PAGE_LIMIT as usize)
//                 .cloned()
//                 .map(|identity| (identity, RewardedSetNodeStatus::Active))
//                 .collect::<Vec<_>>(),
//             start_next_after: Some(format!(
//                 "identity{:04}",
//                 storage::REWARDED_NODE_DEFAULT_PAGE_LIMIT - 1
//             )),
//             at_height: different_height,
//         };
//         assert_eq!(
//             Ok(expected3),
//             query_rewarded_set(deps.as_ref().storage, Some(different_height), None, None)
//         );
//
//         // limit cannot be larger that pre-defined maximum
//         let expected4 = PagedRewardedSetResponse {
//             identities: identities3
//                 .iter()
//                 .take(storage::REWARDED_NODE_MAX_PAGE_LIMIT as usize)
//                 .cloned()
//                 .map(|identity| (identity, RewardedSetNodeStatus::Active))
//                 .collect::<Vec<_>>(),
//             start_next_after: Some(format!(
//                 "identity{:04}",
//                 storage::REWARDED_NODE_MAX_PAGE_LIMIT - 1
//             )),
//             at_height: different_height,
//         };
//         assert_eq!(
//             Ok(expected4),
//             query_rewarded_set(
//                 deps.as_ref().storage,
//                 Some(different_height),
//                 None,
//                 Some(REWARDED_NODE_MAX_PAGE_LIMIT * 100)
//             )
//         );
//     }
//
//     #[test]
//     fn querying_for_rewarded_set_update_details() {
//         let env = mock_env();
//         let mut deps = test_helpers::init_contract();
//
//         let current_height = 123;
//         storage::CURRENT_REWARDED_SET_HEIGHT
//             .save(deps.as_mut().storage, &current_height)
//             .unwrap();
//
//         // returns whatever is in the correct environment
//         assert_eq!(
//             RewardedSetUpdateDetails {
//                 refresh_rate_blocks: crate::constants::REWARDED_SET_REFRESH_BLOCKS,
//                 last_refreshed_block: current_height,
//                 current_height: env.block.height
//             },
//             query_rewarded_set_update_details(env, deps.as_ref().storage).unwrap()
//         )
//     }
// }

// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealings::storage::{StoredDealing, DEALINGS_METADATA};
use cosmwasm_std::{Deps, StdResult};
use nym_coconut_dkg_common::dealing::{
    DealingChunkResponse, DealingChunkStatusResponse, DealingMetadataResponse,
    DealingStatusResponse,
};
use nym_coconut_dkg_common::types::{ChunkIndex, DealingIndex, EpochId};

/// Get the metadata associated with the particular dealing
pub fn query_dealing_metadata(
    deps: Deps<'_>,
    epoch_id: EpochId,
    dealer: String,
    dealing_index: DealingIndex,
) -> StdResult<DealingMetadataResponse> {
    let dealer = deps.api.addr_validate(&dealer)?;
    let metadata = DEALINGS_METADATA.may_load(deps.storage, (epoch_id, &dealer, dealing_index))?;

    Ok(DealingMetadataResponse {
        epoch_id,
        dealer,
        dealing_index,
        metadata,
    })
}

/// Get the status of particular dealing, i.e. whether it has been fully submitted.
pub fn query_dealing_status(
    deps: Deps<'_>,
    epoch_id: EpochId,
    dealer: String,
    dealing_index: DealingIndex,
) -> StdResult<DealingStatusResponse> {
    let dealer = deps.api.addr_validate(&dealer)?;
    let metadata = DEALINGS_METADATA.may_load(deps.storage, (epoch_id, &dealer, dealing_index))?;

    let full_dealing_submitted = if let Some(metadata) = metadata {
        metadata.is_complete()
    } else {
        false
    };

    Ok(DealingStatusResponse {
        epoch_id,
        dealer,
        dealing_index,
        full_dealing_submitted,
    })
}

/// Get the status of particular chunk, i.e. whether (and when) it has been fully submitted.
pub fn query_dealing_chunk_status(
    deps: Deps<'_>,
    epoch_id: EpochId,
    dealer: String,
    dealing_index: DealingIndex,
    chunk_index: ChunkIndex,
) -> StdResult<DealingChunkStatusResponse> {
    let dealer = deps.api.addr_validate(&dealer)?;
    let metadata = DEALINGS_METADATA.may_load(deps.storage, (epoch_id, &dealer, dealing_index))?;

    let submission_height = if let Some(metadata) = metadata {
        if let Some(chunk) = metadata.submitted_chunks.get(&chunk_index) {
            chunk.submission_height
        } else {
            None
        }
    } else {
        None
    };

    Ok(DealingChunkStatusResponse {
        epoch_id,
        dealer,
        dealing_index,
        chunk_index,
        submission_height,
    })
}

/// Get the particular chunk of the dealing.
pub fn query_dealing_chunk(
    deps: Deps<'_>,
    epoch_id: EpochId,
    dealer: String,
    dealing_index: DealingIndex,
    chunk_index: ChunkIndex,
) -> StdResult<DealingChunkResponse> {
    let dealer = deps.api.addr_validate(&dealer)?;
    let chunk = StoredDealing::read(deps.storage, epoch_id, &dealer, dealing_index, chunk_index);

    Ok(DealingChunkResponse {
        epoch_id,
        dealer,
        dealing_index,
        chunk_index,
        chunk,
    })
}

// #[cfg(test)]
// pub(crate) mod tests {
//     use super::*;
//     use crate::dealings::storage::{DEALINGS_PAGE_DEFAULT_LIMIT, DEALINGS_PAGE_MAX_LIMIT};
//     use crate::support::tests::fixtures::{dealing_bytes_fixture, partial_dealing_fixture};
//     use crate::support::tests::helpers::init_contract;
//     use cosmwasm_std::{Addr, DepsMut};
//     use nym_coconut_dkg_common::types::PartialContractDealing;
//
//     fn fill_dealings(deps: DepsMut<'_>, epoch: EpochId, dealers: usize, key_size: u32) {
//         for i in 0..dealers {
//             let dealer = Addr::unchecked(format!("dealer{i}"));
//             for dealing_index in 0..key_size {
//                 StoredDealing::save(
//                     deps.storage,
//                     epoch,
//                     &dealer,
//                     PartialContractDealing {
//                         dealing_index: dealing_index,
//                         data: dealing_bytes_fixture(),
//                     },
//                 )
//             }
//         }
//     }
//
//     #[test]
//     fn test_query_dealing() {
//         let mut deps = init_contract();
//
//         let bad_address = "FOOMP".to_string();
//         assert!(query_dealing(deps.as_ref(), 0, bad_address, 0).is_err());
//
//         let empty = query_dealing(deps.as_ref(), 0, "foo".to_string(), 0).unwrap();
//         assert_eq!(empty.epoch_id, 0);
//         assert_eq!(empty.dealing_index, 0);
//         assert_eq!(empty.dealer, Addr::unchecked("foo"));
//         assert!(empty.dealing.is_none());
//
//         // insert the dealing
//         let dealing = partial_dealing_fixture();
//         StoredDealing::save(
//             deps.as_mut().storage,
//             0,
//             &Addr::unchecked("foo"),
//             dealing.clone(),
//         );
//
//         let retrieved = query_dealing(deps.as_ref(), 0, "foo".to_string(), 0).unwrap();
//         assert_eq!(retrieved.epoch_id, 0);
//         assert_eq!(retrieved.dealing_index, dealing.dealing_index);
//         assert_eq!(retrieved.dealer, Addr::unchecked("foo"));
//         assert_eq!(retrieved.dealing.unwrap(), dealing.data);
//     }
//
//     #[test]
//     fn test_query_dealing_status() {
//         let mut deps = init_contract();
//
//         let bad_address = "FOOMP".to_string();
//         assert!(query_dealing_status(deps.as_ref(), 0, bad_address, 0).is_err());
//
//         let empty = query_dealing_status(deps.as_ref(), 0, "foo".to_string(), 0).unwrap();
//         assert_eq!(empty.epoch_id, 0);
//         assert_eq!(empty.dealing_index, 0);
//         assert_eq!(empty.dealer, Addr::unchecked("foo"));
//         assert!(!empty.dealing_submitted);
//
//         // insert the dealing
//         let dealing = partial_dealing_fixture();
//         StoredDealing::save(
//             deps.as_mut().storage,
//             0,
//             &Addr::unchecked("foo"),
//             dealing.clone(),
//         );
//
//         let retrieved = query_dealing_status(deps.as_ref(), 0, "foo".to_string(), 0).unwrap();
//         assert_eq!(retrieved.epoch_id, 0);
//         assert_eq!(retrieved.dealing_index, dealing.dealing_index);
//         assert_eq!(retrieved.dealer, Addr::unchecked("foo"));
//         assert!(retrieved.dealing_submitted)
//     }
//
//     #[cfg(test)]
//     mod query_dealings {
//         use super::*;
//         use nym_coconut_dkg_common::types::DEFAULT_DEALINGS;
//
//         #[test]
//         fn dealings_empty_on_init() {
//             let deps = init_contract();
//             let all_dealings = StoredDealing::unchecked_all_entries(&deps.storage);
//             assert!(all_dealings.is_empty())
//         }
//
//         #[test]
//         fn dealings_paged_retrieval_obeys_limits() {
//             let mut deps = init_contract();
//             let limit = 2;
//             fill_dealings(deps.as_mut(), 0, 10, DEFAULT_DEALINGS as u32);
//
//             for dealer in 0..10 {
//                 let dealer = format!("dealer{dealer}");
//                 let page1 =
//                     query_dealings_paged(deps.as_ref(), 0, dealer, None, Option::from(limit))
//                         .unwrap();
//                 assert_eq!(limit, page1.dealings.len() as u32);
//             }
//         }
//
//         #[test]
//         fn dealings_paged_retrieval_has_default_limit() {
//             let mut deps = init_contract();
//             fill_dealings(deps.as_mut(), 0, 10, DEFAULT_DEALINGS as u32);
//
//             for dealer in 0..10 {
//                 let dealer = format!("dealer{dealer}");
//                 // query without explicitly setting a limit
//                 let page1 = query_dealings_paged(deps.as_ref(), 0, dealer, None, None).unwrap();
//
//                 assert_eq!(DEALINGS_PAGE_DEFAULT_LIMIT, page1.dealings.len() as u32);
//             }
//         }
//
//         #[test]
//         fn dealings_paged_retrieval_has_max_limit() {
//             let mut deps = init_contract();
//             fill_dealings(deps.as_mut(), 0, 10, DEFAULT_DEALINGS as u32);
//
//             // query with a crazily high limit in an attempt to use too many resources
//             let crazy_limit = 1000 * DEALINGS_PAGE_MAX_LIMIT;
//             for dealer in 0..10 {
//                 let dealer = format!("dealer{dealer}");
//                 let page1 =
//                     query_dealings_paged(deps.as_ref(), 0, dealer, None, Option::from(crazy_limit))
//                         .unwrap();
//
//                 // we default to a decent sized upper bound instead
//                 let expected_limit = DEALINGS_PAGE_MAX_LIMIT;
//                 assert_eq!(expected_limit, page1.dealings.len() as u32);
//             }
//         }
//
//         #[test]
//         fn dealings_pagination_works() {
//             let mut deps = init_contract();
//
//             fill_dealings(deps.as_mut(), 0, 10, 1);
//             let per_page = 2;
//
//             for dealer in 0..10 {
//                 let dealer = format!("dealer{dealer}");
//                 let page1 =
//                     query_dealings_paged(deps.as_ref(), 0, dealer, None, Option::from(per_page))
//                         .unwrap();
//
//                 // page should have 1 result on it
//                 assert_eq!(1, page1.dealings.len());
//             }
//
//             // save another
//             fill_dealings(deps.as_mut(), 1, 10, 2);
//
//             for dealer in 0..10 {
//                 let dealer = format!("dealer{dealer}");
//                 // page1 should have 2 results on it
//                 let page1 =
//                     query_dealings_paged(deps.as_ref(), 1, dealer, None, Option::from(per_page))
//                         .unwrap();
//                 assert_eq!(2, page1.dealings.len());
//             }
//
//             fill_dealings(deps.as_mut(), 3, 10, 3);
//
//             for dealer in 0..10 {
//                 let dealer = format!("dealer{dealer}");
//                 // page1 still has 2 results
//                 let page1 = query_dealings_paged(
//                     deps.as_ref(),
//                     3,
//                     dealer.clone(),
//                     None,
//                     Option::from(per_page),
//                 )
//                 .unwrap();
//                 assert_eq!(2, page1.dealings.len());
//
//                 // retrieving the next page should start after the last key on this page
//                 let start_after = page1.start_next_after.unwrap();
//                 let page2 = query_dealings_paged(
//                     deps.as_ref(),
//                     3,
//                     dealer,
//                     Option::from(start_after),
//                     Option::from(per_page),
//                 )
//                 .unwrap();
//
//                 assert_eq!(1, page2.dealings.len());
//             }
//
//             fill_dealings(deps.as_mut(), 4, 10, 4);
//
//             for dealer in 0..10 {
//                 let dealer = format!("dealer{dealer}");
//                 let page1 = query_dealings_paged(
//                     deps.as_ref(),
//                     4,
//                     dealer.clone(),
//                     None,
//                     Option::from(per_page),
//                 )
//                 .unwrap();
//                 let start_after = page1.start_next_after.unwrap();
//                 let page2 = query_dealings_paged(
//                     deps.as_ref(),
//                     4,
//                     dealer,
//                     Option::from(start_after),
//                     Option::from(per_page),
//                 )
//                 .unwrap();
//
//                 // now we have 2 pages, with 2 results on the second page
//                 assert_eq!(2, page2.dealings.len());
//             }
//         }
//     }
// }

// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage::{self};
use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;
use nym_coconut_dkg_common::dealer::{DealerDetailsResponse, DealerType, PagedDealerResponse};

// fn query_dealers(
//     deps: Deps<'_>,
//     start_after: Option<String>,
//     limit: Option<u32>,
//     underlying_map: &IndexedDealersMap<'_>,
// ) -> StdResult<PagedDealerResponse> {
//     let limit = limit
//         .unwrap_or(storage::DEALERS_PAGE_DEFAULT_LIMIT)
//         .min(storage::DEALERS_PAGE_MAX_LIMIT) as usize;
//
//     let addr = start_after
//         .map(|addr| deps.api.addr_validate(&addr))
//         .transpose()?;
//
//     let start = addr.as_ref().map(Bound::exclusive);
//
//     let dealers = underlying_map
//         .range(deps.storage, start, None, Order::Ascending)
//         .take(limit)
//         .map(|res| res.map(|item| item.1))
//         .collect::<StdResult<Vec<_>>>()?;
//
//     let start_next_after = dealers.last().map(|dealer| dealer.address.clone());
//
//     Ok(PagedDealerResponse::new(dealers, limit, start_next_after))
// }

pub fn query_dealer_details(
    deps: Deps<'_>,
    dealer_address: String,
) -> StdResult<DealerDetailsResponse> {
    let addr = deps.api.addr_validate(&dealer_address)?;

    todo!()
    // if let Some(current) = storage::current_dealers().may_load(deps.storage, &addr)? {
    //     return Ok(DealerDetailsResponse::new(
    //         Some(current),
    //         DealerType::Current,
    //     ));
    // }
    // if let Some(past) = storage::past_dealers().may_load(deps.storage, &addr)? {
    //     return Ok(DealerDetailsResponse::new(Some(past), DealerType::Past));
    // }
    // Ok(DealerDetailsResponse::new(None, DealerType::Unknown))
}

pub fn query_current_dealers_paged(
    deps: Deps<'_>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PagedDealerResponse> {
    todo!()
    // query_dealers(deps, start_after, limit, &storage::current_dealers())
}

pub fn query_past_dealers_paged(
    deps: Deps<'_>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PagedDealerResponse> {
    todo!()
    // query_dealers(deps, start_after, limit, &storage::past_dealers())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::dealers::storage::{DEALERS_PAGE_DEFAULT_LIMIT, DEALERS_PAGE_MAX_LIMIT};
    use crate::support::tests::fixtures::dealer_details_fixture;
    use crate::support::tests::helpers::init_contract;
    use cosmwasm_std::DepsMut;

    fn fill_dealers(deps: DepsMut<'_>, mapping: &IndexedDealersMap<'_>, size: usize) {
        for n in 0..size {
            let dealer_details = dealer_details_fixture(n as u64);
            mapping
                .save(deps.storage, &dealer_details.address, &dealer_details)
                .unwrap();
        }
    }

    fn remove_dealers(deps: DepsMut<'_>, mapping: &IndexedDealersMap<'_>, size: usize) {
        for n in 0..size {
            let dealer_details = dealer_details_fixture(n as u64);
            mapping
                .remove(deps.storage, &dealer_details.address)
                .unwrap();
        }
    }

    #[test]
    fn dealers_empty_on_init() {
        let deps = init_contract();

        for mapping in [storage::current_dealers(), storage::past_dealers()] {
            let page1 = query_dealers(deps.as_ref(), None, None, &mapping).unwrap();
            assert_eq!(0, page1.dealers.len() as u32);
        }
    }

    #[test]
    fn dealers_paged_retrieval_obeys_limits() {
        let mut deps = init_contract();
        let limit = 2;

        for mapping in [storage::current_dealers(), storage::past_dealers()] {
            fill_dealers(deps.as_mut(), &mapping, 1000);

            let page1 = query_dealers(deps.as_ref(), None, Option::from(limit), &mapping).unwrap();
            assert_eq!(limit, page1.dealers.len() as u32);

            remove_dealers(deps.as_mut(), &mapping, 1000);
        }
    }

    #[test]
    fn dealers_paged_retrieval_has_default_limit() {
        let mut deps = init_contract();

        for mapping in [storage::current_dealers(), storage::past_dealers()] {
            fill_dealers(deps.as_mut(), &mapping, 1000);

            // query without explicitly setting a limit
            let page1 = query_dealers(deps.as_ref(), None, None, &mapping).unwrap();

            assert_eq!(DEALERS_PAGE_DEFAULT_LIMIT, page1.dealers.len() as u32);

            remove_dealers(deps.as_mut(), &mapping, 1000);
        }
    }

    #[test]
    fn dealers_paged_retrieval_has_max_limit() {
        let mut deps = init_contract();

        // query with a crazily high limit in an attempt to use too many resources
        let crazy_limit = 1000 * DEALERS_PAGE_MAX_LIMIT;

        for mapping in [storage::current_dealers(), storage::past_dealers()] {
            fill_dealers(deps.as_mut(), &mapping, 1000);

            let page1 =
                query_dealers(deps.as_ref(), None, Option::from(crazy_limit), &mapping).unwrap();

            // we default to a decent sized upper bound instead
            let expected_limit = DEALERS_PAGE_MAX_LIMIT;
            assert_eq!(expected_limit, page1.dealers.len() as u32);

            remove_dealers(deps.as_mut(), &mapping, 1000);
        }
    }

    #[test]
    fn dealers_pagination_works() {
        let mut deps = init_contract();

        let per_page = 2;

        for mapping in [storage::current_dealers(), storage::past_dealers()] {
            fill_dealers(deps.as_mut(), &mapping, 1);
            let page1 =
                query_dealers(deps.as_ref(), None, Option::from(per_page), &mapping).unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.dealers.len());
            remove_dealers(deps.as_mut(), &mapping, 1);
        }

        for mapping in [storage::current_dealers(), storage::past_dealers()] {
            fill_dealers(deps.as_mut(), &mapping, 2);
            // page1 should have 2 results on it
            let page1 =
                query_dealers(deps.as_ref(), None, Option::from(per_page), &mapping).unwrap();
            assert_eq!(2, page1.dealers.len());
            remove_dealers(deps.as_mut(), &mapping, 2);
        }

        for mapping in [storage::current_dealers(), storage::past_dealers()] {
            fill_dealers(deps.as_mut(), &mapping, 3);
            // page1 still has 2 results
            let page1 =
                query_dealers(deps.as_ref(), None, Option::from(per_page), &mapping).unwrap();
            assert_eq!(2, page1.dealers.len());

            // retrieving the next page should start after the last key on this page
            let start_after = page1.start_next_after.unwrap();
            let page2 = query_dealers(
                deps.as_ref(),
                Option::from(start_after.to_string()),
                Option::from(per_page),
                &mapping,
            )
            .unwrap();

            assert_eq!(1, page2.dealers.len());
            remove_dealers(deps.as_mut(), &mapping, 3);
        }

        for mapping in [storage::current_dealers(), storage::past_dealers()] {
            fill_dealers(deps.as_mut(), &mapping, 4);
            let page1 =
                query_dealers(deps.as_ref(), None, Option::from(per_page), &mapping).unwrap();
            let start_after = page1.start_next_after.unwrap();
            let page2 = query_dealers(
                deps.as_ref(),
                Option::from(start_after.to_string()),
                Option::from(per_page),
                &mapping,
            )
            .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.dealers.len());
            remove_dealers(deps.as_mut(), &mapping, 4);
        }
    }
}

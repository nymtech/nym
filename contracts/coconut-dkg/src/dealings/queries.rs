// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealings::storage;
use crate::dealings::storage::StoredDealing;
use cosmwasm_std::{Deps, StdResult};
use cw_storage_plus::Bound;
use nym_coconut_dkg_common::dealer::{DealingResponse, PagedDealingsResponse};
use nym_coconut_dkg_common::types::{DealingIndex, EpochId};
pub fn query_dealing(
    deps: Deps<'_>,
    epoch_id: EpochId,
    dealer: String,
    dealing_index: DealingIndex,
) -> StdResult<DealingResponse> {
    let dealer = deps.api.addr_validate(&dealer)?;
    let dealing = StoredDealing::read(deps.storage, epoch_id, &dealer, dealing_index);

    Ok(DealingResponse {
        epoch_id,
        dealer,
        dealing_index,
        dealing,
    })
}

pub fn query_dealings_paged(
    deps: Deps<'_>,
    epoch_id: EpochId,
    dealer: String,
    start_after: Option<DealingIndex>,
    limit: Option<u32>,
) -> StdResult<PagedDealingsResponse> {
    let limit = limit
        .unwrap_or(storage::DEALINGS_PAGE_DEFAULT_LIMIT)
        .min(storage::DEALINGS_PAGE_MAX_LIMIT);

    let dealer = deps.api.addr_validate(&dealer)?;
    let start = start_after.map(Bound::exclusive);

    let dealings = StoredDealing::prefix_range(deps.storage, (epoch_id, &dealer), start)
        .take(limit as usize)
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = dealings.last().map(|dealing| dealing.index);

    Ok(PagedDealingsResponse::new(
        epoch_id,
        dealer,
        dealings,
        start_next_after,
    ))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::dealings::storage::{DEALINGS_PAGE_DEFAULT_LIMIT, DEALINGS_PAGE_MAX_LIMIT};
    use crate::support::tests::fixtures::dealing_bytes_fixture;
    use crate::support::tests::helpers::init_contract;
    use cosmwasm_std::{Addr, DepsMut};

    fn fill_dealings(deps: DepsMut<'_>, size: usize) {
        for n in 0..size {
            let dealing_share = dealing_bytes_fixture();
            let sender = Addr::unchecked(format!("owner{}", n));
            (0..TOTAL_DEALINGS).for_each(|idx| {
                DEALINGS_BYTES[idx]
                    .save(deps.storage, &sender, &dealing_share)
                    .unwrap();
            });
        }
    }

    #[test]
    fn empty_on_bad_idx() {
        let mut deps = init_contract();
        fill_dealings(deps.as_mut(), 1000);

        for idx in TOTAL_DEALINGS as u64..100 * TOTAL_DEALINGS as u64 {
            let page1 = query_dealings_paged(deps.as_ref(), idx, None, None).unwrap();
            assert_eq!(0, page1.dealings.len() as u32);
        }
    }

    #[test]
    fn dealings_empty_on_init() {
        let deps = init_contract();
        for idx in 0..TOTAL_DEALINGS as u64 {
            let response = query_dealings_paged(deps.as_ref(), idx, None, Option::from(2)).unwrap();
            assert_eq!(0, response.dealings.len());
        }
    }

    #[test]
    fn dealings_paged_retrieval_obeys_limits() {
        let mut deps = init_contract();
        let limit = 2;
        fill_dealings(deps.as_mut(), 1000);

        for idx in 0..TOTAL_DEALINGS as u64 {
            let page1 =
                query_dealings_paged(deps.as_ref(), idx, None, Option::from(limit)).unwrap();
            assert_eq!(limit, page1.dealings.len() as u32);
        }
    }

    #[test]
    fn dealings_paged_retrieval_has_default_limit() {
        let mut deps = init_contract();
        fill_dealings(deps.as_mut(), 1000);

        for idx in 0..TOTAL_DEALINGS as u64 {
            // query without explicitly setting a limit
            let page1 = query_dealings_paged(deps.as_ref(), idx, None, None).unwrap();

            assert_eq!(DEALINGS_PAGE_DEFAULT_LIMIT, page1.dealings.len() as u32);
        }
    }

    #[test]
    fn dealings_paged_retrieval_has_max_limit() {
        let mut deps = init_contract();
        fill_dealings(deps.as_mut(), 1000);

        // query with a crazily high limit in an attempt to use too many resources
        let crazy_limit = 1000 * DEALINGS_PAGE_MAX_LIMIT;
        for idx in 0..TOTAL_DEALINGS as u64 {
            let page1 =
                query_dealings_paged(deps.as_ref(), idx, None, Option::from(crazy_limit)).unwrap();

            // we default to a decent sized upper bound instead
            let expected_limit = DEALINGS_PAGE_MAX_LIMIT;
            assert_eq!(expected_limit, page1.dealings.len() as u32);
        }
    }

    #[test]
    fn dealings_pagination_works() {
        let mut deps = init_contract();

        fill_dealings(deps.as_mut(), 1);

        let per_page = 2;

        for idx in 0..TOTAL_DEALINGS as u64 {
            let page1 =
                query_dealings_paged(deps.as_ref(), idx, None, Option::from(per_page)).unwrap();

            // page should have 1 result on it
            assert_eq!(1, page1.dealings.len());
        }

        // save another
        fill_dealings(deps.as_mut(), 2);

        for idx in 0..TOTAL_DEALINGS as u64 {
            // page1 should have 2 results on it
            let page1 =
                query_dealings_paged(deps.as_ref(), idx, None, Option::from(per_page)).unwrap();
            assert_eq!(2, page1.dealings.len());
        }

        fill_dealings(deps.as_mut(), 3);

        for idx in 0..TOTAL_DEALINGS as u64 {
            // page1 still has 2 results
            let page1 =
                query_dealings_paged(deps.as_ref(), idx, None, Option::from(per_page)).unwrap();
            assert_eq!(2, page1.dealings.len());

            // retrieving the next page should start after the last key on this page
            let start_after = page1.start_next_after.unwrap();
            let page2 = query_dealings_paged(
                deps.as_ref(),
                idx,
                Option::from(start_after.to_string()),
                Option::from(per_page),
            )
            .unwrap();

            assert_eq!(1, page2.dealings.len());
        }

        fill_dealings(deps.as_mut(), 4);

        for idx in 0..TOTAL_DEALINGS as u64 {
            let page1 =
                query_dealings_paged(deps.as_ref(), idx, None, Option::from(per_page)).unwrap();
            let start_after = page1.start_next_after.unwrap();
            let page2 = query_dealings_paged(
                deps.as_ref(),
                idx,
                Option::from(start_after.to_string()),
                Option::from(per_page),
            )
            .unwrap();

            // now we have 2 pages, with 2 results on the second page
            assert_eq!(2, page2.dealings.len());
        }
    }
}

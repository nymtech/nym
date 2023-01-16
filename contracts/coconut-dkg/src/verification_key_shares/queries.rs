// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::verification_key_shares::storage;
use crate::verification_key_shares::storage::vk_shares;
use coconut_dkg_common::verification_key::PagedVKSharesResponse;
use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;

pub fn query_vk_shares_paged(
    deps: Deps<'_>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PagedVKSharesResponse> {
    let limit = limit
        .unwrap_or(storage::VERIFICATION_KEY_SHARES_PAGE_DEFAULT_LIMIT)
        .min(storage::VERIFICATION_KEY_SHARES_PAGE_MAX_LIMIT) as usize;

    let addr = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;

    let start = addr.as_ref().map(Bound::exclusive);

    let shares = vk_shares()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|(_, share)| share))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = shares.last().map(|share| share.owner.clone());

    Ok(PagedVKSharesResponse {
        shares,
        per_page: limit,
        start_next_after,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::support::tests::fixtures::vk_share_fixture;
    use crate::support::tests::helpers::init_contract;
    use crate::verification_key_shares::storage::{
        VERIFICATION_KEY_SHARES_PAGE_DEFAULT_LIMIT, VERIFICATION_KEY_SHARES_PAGE_MAX_LIMIT,
    };
    use cosmwasm_std::Addr;

    #[test]
    fn vk_shares_empty_on_init() {
        let deps = init_contract();
        let response = query_vk_shares_paged(deps.as_ref(), None, Option::from(2)).unwrap();
        assert_eq!(0, response.shares.len());
    }

    #[test]
    fn vk_shares_paged_retrieval_obeys_limits() {
        let mut deps = init_contract();
        let limit = 2;
        for n in 0..1000 {
            let vk_share = vk_share_fixture(n);
            let sender = Addr::unchecked(format!("owner{}", n));
            vk_shares()
                .save(&mut deps.storage, &sender, &vk_share)
                .unwrap();
        }

        let page1 = query_vk_shares_paged(deps.as_ref(), None, Option::from(limit)).unwrap();
        assert_eq!(limit, page1.shares.len() as u32);
    }

    #[test]
    fn vk_shares_paged_retrieval_has_default_limit() {
        let mut deps = init_contract();
        for n in 0..1000 {
            let vk_share = vk_share_fixture(n);
            let sender = Addr::unchecked(format!("owner{}", n));
            vk_shares()
                .save(&mut deps.storage, &sender, &vk_share)
                .unwrap();
        }

        // query without explicitly setting a limit
        let page1 = query_vk_shares_paged(deps.as_ref(), None, None).unwrap();

        assert_eq!(
            VERIFICATION_KEY_SHARES_PAGE_DEFAULT_LIMIT,
            page1.shares.len() as u32
        );
    }

    #[test]
    fn vk_shares_paged_retrieval_has_max_limit() {
        let mut deps = init_contract();
        for n in 0..1000 {
            let vk_share = vk_share_fixture(n);
            let sender = Addr::unchecked(format!("owner{}", n));
            vk_shares()
                .save(&mut deps.storage, &sender, &vk_share)
                .unwrap();
        }

        // query with a crazily high limit in an attempt to use too many resources
        let crazy_limit = 1000 * VERIFICATION_KEY_SHARES_PAGE_MAX_LIMIT;
        let page1 = query_vk_shares_paged(deps.as_ref(), None, Option::from(crazy_limit)).unwrap();

        // we default to a decent sized upper bound instead
        let expected_limit = VERIFICATION_KEY_SHARES_PAGE_MAX_LIMIT;
        assert_eq!(expected_limit, page1.shares.len() as u32);
    }

    #[test]
    fn vk_shares_pagination_works() {
        let mut deps = init_contract();

        let vk_share = vk_share_fixture(1);
        let sender = Addr::unchecked(format!("owner{}", 1));
        vk_shares()
            .save(&mut deps.storage, &sender, &vk_share)
            .unwrap();

        let per_page = 2;
        let page1 = query_vk_shares_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();

        // page should have 1 result on it
        assert_eq!(1, page1.shares.len());

        // save another
        let vk_share = vk_share_fixture(2);
        let sender = Addr::unchecked(format!("owner{}", 2));
        vk_shares()
            .save(&mut deps.storage, &sender, &vk_share)
            .unwrap();

        // page1 should have 2 results on it
        let page1 = query_vk_shares_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.shares.len());

        let vk_share = vk_share_fixture(3);
        let sender = Addr::unchecked(format!("owner{}", 3));
        vk_shares()
            .save(&mut deps.storage, &sender, &vk_share)
            .unwrap();

        // page1 still has 2 results
        let page1 = query_vk_shares_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.shares.len());

        // retrieving the next page should start after the last key on this page
        let start_after = page1.start_next_after.unwrap();
        let page2 = query_vk_shares_paged(
            deps.as_ref(),
            Option::from(start_after.to_string()),
            Option::from(per_page),
        )
        .unwrap();

        assert_eq!(1, page2.shares.len());

        let vk_share = vk_share_fixture(4);
        let sender = Addr::unchecked(format!("owner{}", 4));
        vk_shares()
            .save(&mut deps.storage, &sender, &vk_share)
            .unwrap();

        let page2 = query_vk_shares_paged(
            deps.as_ref(),
            Option::from(start_after.to_string()),
            Option::from(per_page),
        )
        .unwrap();

        // now we have 2 pages, with 2 results on the second page
        assert_eq!(2, page2.shares.len());
    }
}

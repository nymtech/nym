// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage;
use crate::dealers::storage::DealersMap;
use coconut_dkg_common::dealer::PagedDealerResponse;
use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;

fn query_dealers(
    deps: Deps<'_>,
    start_after: Option<String>,
    limit: Option<u32>,
    underlying_map: DealersMap<'_>,
) -> StdResult<PagedDealerResponse> {
    let limit = limit
        .unwrap_or(storage::DEALERS_PAGE_DEFAULT_LIMIT)
        .min(storage::DEALERS_PAGE_MAX_LIMIT) as usize;

    let addr = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;

    let start = addr.as_ref().map(Bound::exclusive);

    let dealers = underlying_map
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = dealers.last().map(|dealer| dealer.address.clone());

    Ok(PagedDealerResponse::new(dealers, limit, start_next_after))
}

pub fn query_current_dealers_paged(
    deps: Deps<'_>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PagedDealerResponse> {
    query_dealers(deps, start_after, limit, storage::current_dealers())
}

pub fn query_past_dealers_paged(
    deps: Deps<'_>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PagedDealerResponse> {
    query_dealers(deps, start_after, limit, storage::past_dealers())
}

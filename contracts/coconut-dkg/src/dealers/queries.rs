// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealers::storage;
use crate::dealers::storage::{IndexedDealersMap, BLACKLISTED_DEALERS};
use coconut_dkg_common::dealer::{
    BlacklistedDealer, BlacklistingResponse, DealerDetailsResponse, DealerType,
    PagedBlacklistingResponse, PagedDealerResponse,
};
use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;

fn query_dealers(
    deps: Deps<'_>,
    start_after: Option<String>,
    limit: Option<u32>,
    underlying_map: IndexedDealersMap<'_>,
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

pub fn query_dealer_details(
    deps: Deps<'_>,
    dealer_address: String,
) -> StdResult<DealerDetailsResponse> {
    let addr = deps.api.addr_validate(&dealer_address)?;
    if let Some(current) = storage::current_dealers().may_load(deps.storage, &addr)? {
        return Ok(DealerDetailsResponse::new(
            Some(current),
            DealerType::Current,
        ));
    }
    if let Some(past) = storage::past_dealers().may_load(deps.storage, &addr)? {
        return Ok(DealerDetailsResponse::new(Some(past), DealerType::Past));
    }
    Ok(DealerDetailsResponse::new(None, DealerType::Unknown))
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

pub fn query_blacklisted_dealers_paged(
    deps: Deps<'_>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PagedBlacklistingResponse> {
    let limit = limit
        .unwrap_or(storage::DEALERS_PAGE_DEFAULT_LIMIT)
        .min(storage::DEALERS_PAGE_MAX_LIMIT) as usize;

    let addr = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;

    let start = addr.as_ref().map(Bound::exclusive);

    let blacklisted_dealers = BLACKLISTED_DEALERS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|(addr, blacklisting)| BlacklistedDealer::new(addr, blacklisting)))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = blacklisted_dealers
        .last()
        .map(|dealer| dealer.dealer.clone());

    Ok(PagedBlacklistingResponse::new(
        blacklisted_dealers,
        limit,
        start_next_after,
    ))
}

pub fn query_blacklisting(deps: Deps<'_>, dealer: String) -> StdResult<BlacklistingResponse> {
    let addr = deps.api.addr_validate(&dealer)?;
    let blacklisting = BLACKLISTED_DEALERS.may_load(deps.storage, &addr)?;
    Ok(BlacklistingResponse::new(addr, blacklisting))
}

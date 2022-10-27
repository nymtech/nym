// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealings::storage;
use crate::dealings::storage::DEALINGS_BYTES;
use coconut_dkg_common::dealer::{ContractDealing, PagedDealingsResponse};
use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;

pub fn query_dealings_paged(
    deps: Deps<'_>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PagedDealingsResponse> {
    let limit = limit
        .unwrap_or(storage::DEALINGS_PAGE_DEFAULT_LIMIT)
        .min(storage::DEALINGS_PAGE_MAX_LIMIT) as usize;

    let addr = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;

    let start = addr.as_ref().map(Bound::exclusive);

    let dealings = DEALINGS_BYTES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|(dealer, dealings)| ContractDealing::new(dealings, dealer)))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = dealings.last().map(|commitment| commitment.dealer.clone());

    Ok(PagedDealingsResponse::new(
        dealings,
        limit,
        start_next_after,
    ))
}

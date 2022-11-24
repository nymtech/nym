// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dealings::storage;
use crate::dealings::storage::DEALINGS_BYTES;
use coconut_dkg_common::dealer::{ContractDealing, PagedDealingsResponse};
use coconut_dkg_common::types::TOTAL_DEALINGS;
use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;

pub fn query_dealings_paged(
    deps: Deps<'_>,
    idx: u64,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PagedDealingsResponse> {
    let limit = limit
        .unwrap_or(storage::DEALINGS_PAGE_DEFAULT_LIMIT)
        .min(storage::DEALINGS_PAGE_MAX_LIMIT) as usize;

    let idx = idx as usize;
    if idx >= TOTAL_DEALINGS {
        return Ok(PagedDealingsResponse::new(vec![], limit, None));
    }

    let addr = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;

    let start = addr.as_ref().map(Bound::exclusive);

    let dealings = DEALINGS_BYTES[idx]
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|(dealer, dealing)| ContractDealing::new(dealing, dealer)))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = dealings.last().map(|dealing| dealing.dealer.clone());

    Ok(PagedDealingsResponse::new(
        dealings,
        limit,
        start_next_after,
    ))
}

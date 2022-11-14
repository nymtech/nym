// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::verification_key_shares::storage;
use crate::verification_key_shares::storage::VK_SHARES;
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

    let shares = VK_SHARES
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

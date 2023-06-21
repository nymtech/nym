// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::peers::storage::{PEERS, PEERS_PAGE_DEFAULT_LIMIT, PEERS_PAGE_MAX_LIMIT};
use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;
use nym_ephemera_common::peers::PagedPeerResponse;

pub fn query_peers_paged(
    deps: Deps<'_>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<PagedPeerResponse> {
    let limit = limit
        .unwrap_or(PEERS_PAGE_DEFAULT_LIMIT)
        .min(PEERS_PAGE_MAX_LIMIT) as usize;

    let addr = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;

    let start = addr.map(Bound::exclusive);

    let peers = PEERS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = peers
        .last()
        .map(|peer_info| peer_info.cosmos_address.clone());

    Ok(PagedPeerResponse::new(peers, limit, start_next_after))
}

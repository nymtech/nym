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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::peers::storage::{PEERS_PAGE_DEFAULT_LIMIT, PEERS_PAGE_MAX_LIMIT};
    use crate::support::tests::fixtures::peer_fixture;
    use crate::support::tests::helpers::init_contract;
    use cosmwasm_std::DepsMut;

    fn fill_peers(deps: DepsMut<'_>, size: usize) {
        for n in 0..size {
            let peer = peer_fixture(&format!("peer{}", n));
            PEERS
                .save(deps.storage, peer.cosmos_address.clone(), &peer)
                .unwrap();
        }
    }

    fn remove_peers(deps: DepsMut<'_>, size: usize) {
        for n in 0..size {
            let peer = peer_fixture(&format!("peer{}", n));
            PEERS.remove(deps.storage, peer.cosmos_address);
        }
    }

    #[test]
    fn peers_empty_on_init() {
        let deps = init_contract();

        let page1 = query_peers_paged(deps.as_ref(), None, None).unwrap();
        assert_eq!(0, page1.peers.len() as u32);
    }

    #[test]
    fn peers_paged_retrieval_obeys_limits() {
        let mut deps = init_contract();
        let limit = 2;

        fill_peers(deps.as_mut(), 1000);

        let page1 = query_peers_paged(deps.as_ref(), None, Option::from(limit)).unwrap();
        assert_eq!(limit, page1.peers.len() as u32);

        remove_peers(deps.as_mut(), 1000);
    }

    #[test]
    fn peers_paged_retrieval_has_default_limit() {
        let mut deps = init_contract();

        fill_peers(deps.as_mut(), 1000);

        // query without explicitly setting a limit
        let page1 = query_peers_paged(deps.as_ref(), None, None).unwrap();

        assert_eq!(PEERS_PAGE_DEFAULT_LIMIT, page1.peers.len() as u32);

        remove_peers(deps.as_mut(), 1000);
    }

    #[test]
    fn peers_paged_retrieval_has_max_limit() {
        let mut deps = init_contract();

        // query with a crazily high limit in an attempt to use too many resources
        let crazy_limit = 1000 * PEERS_PAGE_MAX_LIMIT;

        fill_peers(deps.as_mut(), 1000);

        let page1 = query_peers_paged(deps.as_ref(), None, Option::from(crazy_limit)).unwrap();

        // we default to a decent sized upper bound instead
        let expected_limit = PEERS_PAGE_MAX_LIMIT;
        assert_eq!(expected_limit, page1.peers.len() as u32);

        remove_peers(deps.as_mut(), 1000);
    }

    #[test]
    fn peers_pagination_works() {
        let mut deps = init_contract();

        let per_page = 2;

        fill_peers(deps.as_mut(), 1);
        let page1 = query_peers_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();

        // page should have 1 result on it
        assert_eq!(1, page1.peers.len());
        remove_peers(deps.as_mut(), 1);

        fill_peers(deps.as_mut(), 2);
        // page1 should have 2 results on it
        let page1 = query_peers_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.peers.len());
        remove_peers(deps.as_mut(), 2);

        fill_peers(deps.as_mut(), 3);
        // page1 still has 2 results
        let page1 = query_peers_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.peers.len());

        // retrieving the next page should start after the last key on this page
        let start_after = page1.start_next_after.unwrap();
        let page2 = query_peers_paged(
            deps.as_ref(),
            Option::from(start_after.to_string()),
            Option::from(per_page),
        )
        .unwrap();

        assert_eq!(1, page2.peers.len());
        remove_peers(deps.as_mut(), 3);

        fill_peers(deps.as_mut(), 4);
        let page1 = query_peers_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        let start_after = page1.start_next_after.unwrap();
        let page2 = query_peers_paged(
            deps.as_ref(),
            Option::from(start_after.to_string()),
            Option::from(per_page),
        )
        .unwrap();

        // now we have 2 pages, with 2 results on the second page
        assert_eq!(2, page2.peers.len());
        remove_peers(deps.as_mut(), 4);
    }
}

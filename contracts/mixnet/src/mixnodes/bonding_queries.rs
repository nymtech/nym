// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::query_support::calculate_start_value;
use config::defaults::DENOM;
use cosmwasm_std::{coin, Addr, Deps, Order, StdResult};
use mixnet_contract::{
    Delegation, IdentityKey, MixNodeBond, MixOwnershipResponse, PagedMixDelegationsResponse,
    PagedMixnodeResponse,
};

pub fn query_mixnodes_paged(
    deps: Deps,
    start_after: Option<IdentityKey>,
    limit: Option<u32>,
) -> StdResult<PagedMixnodeResponse> {
    let limit = limit
        .unwrap_or(storage::BOND_PAGE_DEFAULT_LIMIT)
        .min(storage::BOND_PAGE_MAX_LIMIT) as usize;
    let start = calculate_start_value(start_after);

    let nodes = storage::mixnodes_read(deps.storage)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .map(|stored_bond| {
            // I really don't like this additional read per entry, but I don't see an obvious way to remove it
            stored_bond.map(|stored_bond| {
                let total_delegation = storage::total_delegation_read(deps.storage)
                    .load(stored_bond.identity().as_bytes());
                total_delegation
                    .map(|total_delegation| stored_bond.attach_delegation(total_delegation))
            })
        })
        .collect::<StdResult<StdResult<Vec<MixNodeBond>>>>()??;

    let start_next_after = nodes.last().map(|node| node.identity().clone());

    Ok(PagedMixnodeResponse::new(nodes, limit, start_next_after))
}

pub fn query_owns_mixnode(deps: Deps, address: Addr) -> StdResult<MixOwnershipResponse> {
    let has_node = storage::mixnodes_owners_read(deps.storage)
        .may_load(address.as_bytes())?
        .is_some();
    Ok(MixOwnershipResponse { address, has_node })
}

pub(crate) fn query_mixnode_delegations_paged(
    deps: Deps,
    mix_identity: IdentityKey,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<PagedMixDelegationsResponse> {
    let limit = limit
        .unwrap_or(storage::DELEGATION_PAGE_DEFAULT_LIMIT)
        .min(storage::DELEGATION_PAGE_MAX_LIMIT) as usize;
    let start = calculate_start_value(start_after);

    let delegations = storage::mix_delegations_read(deps.storage, &mix_identity)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|res| {
            res.map(|entry| {
                Delegation::new(
                    Addr::unchecked(String::from_utf8(entry.0).expect(
                        "Non-UTF8 address used as key in bucket. The storage is corrupted!",
                    )),
                    coin(entry.1.amount.u128(), DENOM),
                    entry.1.block_height,
                )
            })
        })
        .collect::<StdResult<Vec<Delegation>>>()?;

    let start_next_after = delegations.last().map(|delegation| delegation.owner());

    Ok(PagedMixDelegationsResponse::new(
        mix_identity,
        delegations,
        start_next_after,
    ))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    use super::storage;
    use crate::support::tests::test_helpers;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::Addr;
    use mixnet_contract::MixNode;

    #[test]
    fn mixnodes_empty_on_init() {
        let deps = test_helpers::init_contract();
        let response = query_mixnodes_paged(deps.as_ref(), None, Option::from(2)).unwrap();
        assert_eq!(0, response.nodes.len());
    }

    #[test]
    fn mixnodes_paged_retrieval_obeys_limits() {
        let mut deps = helpers::init_contract();
        let limit = 2;
        for n in 0..10000 {
            let key = format!("bond{}", n);
            test_helpers::add_mixnode(&key, test_helpers::good_mixnode_bond(), deps.as_mut());
        }

        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(limit)).unwrap();
        assert_eq!(limit, page1.nodes.len() as u32);
    }

    #[test]
    fn mixnodes_paged_retrieval_has_default_limit() {
        let mut deps = helpers::init_contract();
        for n in 0..100 {
            let key = format!("bond{}", n);
            test_helpers::add_mixnode(&key, test_helpers::good_mixnode_bond(), deps.as_mut());
        }

        // query without explicitly setting a limit
        let page1 = query_mixnodes_paged(deps.as_ref(), None, None).unwrap();

        let expected_limit = 50;
        assert_eq!(expected_limit, page1.nodes.len() as u32);
    }

    #[test]
    fn mixnodes_paged_retrieval_has_max_limit() {
        let mut deps = helpers::init_contract();
        for n in 0..10000 {
            let key = format!("bond{}", n);
            test_helpers::add_mixnode(&key, test_helpers::good_mixnode_bond(), deps.as_mut());
        }

        // query with a crazily high limit in an attempt to use too many resources
        let crazy_limit = 1000;
        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(crazy_limit)).unwrap();

        // we default to a decent sized upper bound instead
        let expected_limit = storage::BOND_PAGE_MAX_LIMIT;
        assert_eq!(expected_limit, page1.nodes.len() as u32);
    }

    #[test]
    fn pagination_works() {
        let addr1 = "nym100";
        let addr2 = "nym101";
        let addr3 = "nym102";
        let addr4 = "nym103";

        let mut deps = test_helpers::init_contract();
        let _identity1 =
            test_helpers::add_mixnode(&addr1, test_helpers::good_mixnode_bond(), deps.as_mut());

        let per_page = 2;
        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();

        // page should have 1 result on it
        assert_eq!(1, page1.nodes.len());

        // save another
        let identity2 =
            test_helpers::add_mixnode(&addr2, test_helpers::good_mixnode_bond(), deps.as_mut());

        // page1 should have 2 results on it
        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.nodes.len());

        let _identity3 =
            test_helpers::add_mixnode(&addr3, test_helpers::good_mixnode_bond(), deps.as_mut());

        // page1 still has 2 results
        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.nodes.len());

        // retrieving the next page should start after the last key on this page
        let start_after = identity2.clone();
        let page2 = query_mixnodes_paged(
            deps.as_ref(),
            Option::from(start_after),
            Option::from(per_page),
        )
        .unwrap();

        assert_eq!(1, page2.nodes.len());

        // save another one
        test_helpers::add_mixnode(&addr4, test_helpers::good_mixnode_bond(), deps.as_mut());

        let start_after = identity2;
        let page2 = query_mixnodes_paged(
            deps.as_ref(),
            Option::from(start_after),
            Option::from(per_page),
        )
        .unwrap();

        // now we have 2 pages, with 2 results on the second page
        assert_eq!(2, page2.nodes.len());
    }

    #[test]
    fn query_for_mixnode_owner_works() {
        let mut deps = test_helpers::init_contract();

        // "fred" does not own a mixnode if there are no mixnodes
        let res = query_owns_mixnode(deps.as_ref(), Addr::unchecked("fred")).unwrap();
        assert!(!res.has_node);

        // mixnode was added to "bob", "fred" still does not own one
        let node = MixNode {
            identity_key: "bobsnode".into(),
            ..test_helpers::mix_node_fixture()
        };
        crate::mixnodes::bonding_transactions::try_add_mixnode(
            deps.as_mut(),
            mock_env(),
            mock_info("bob", &test_helpers::good_mixnode_bond()),
            node,
        )
        .unwrap();

        let res = query_owns_mixnode(deps.as_ref(), Addr::unchecked("fred")).unwrap();
        assert!(!res.has_node);

        // "fred" now owns a mixnode!
        let node = MixNode {
            identity_key: "fredsnode".into(),
            ..test_helpers::mix_node_fixture()
        };
        crate::mixnodes::bonding_transactions::try_add_mixnode(
            deps.as_mut(),
            mock_env(),
            mock_info("fred", &test_helpers::good_mixnode_bond()),
            node,
        )
        .unwrap();

        let res = query_owns_mixnode(deps.as_ref(), Addr::unchecked("fred")).unwrap();
        assert!(res.has_node);

        // but after unbonding it, he doesn't own one anymore
        crate::mixnodes::bonding_transactions::try_remove_mixnode(
            deps.as_mut(),
            mock_info("fred", &[]),
        )
        .unwrap();

        let res = query_owns_mixnode(deps.as_ref(), Addr::unchecked("fred")).unwrap();
        assert!(!res.has_node);
    }
}

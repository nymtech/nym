// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;
use mixnet_contract_common::{
    IdentityKey, MixNodeBond, MixOwnershipResponse, PagedMixnodeResponse,
};

pub fn query_mixnodes_paged(
    deps: Deps,
    start_after: Option<IdentityKey>,
    limit: Option<u32>,
) -> StdResult<PagedMixnodeResponse> {
    let limit = limit
        .unwrap_or(storage::BOND_PAGE_DEFAULT_LIMIT)
        .min(storage::BOND_PAGE_MAX_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let nodes = storage::mixnodes()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .map(|stored_bond| {
            // I really don't like this additional read per entry, but I don't see an obvious way to remove it
            stored_bond.map(|stored_bond| {
                let total_delegation =
                    storage::TOTAL_DELEGATION.load(deps.storage, stored_bond.identity());
                total_delegation
                    .map(|total_delegation| stored_bond.attach_delegation(total_delegation))
            })
        })
        .collect::<StdResult<StdResult<Vec<MixNodeBond>>>>()??;

    let start_next_after = nodes.last().map(|node| node.identity().clone());

    Ok(PagedMixnodeResponse::new(nodes, limit, start_next_after))
}

pub fn query_owns_mixnode(deps: Deps, address: String) -> StdResult<MixOwnershipResponse> {
    let validated_addr = deps.api.addr_validate(&address)?;
    let stored_bond = storage::mixnodes()
        .idx
        .owner
        .item(deps.storage, validated_addr.clone())?
        .map(|record| record.1);

    let mixnode = match stored_bond {
        None => None,
        Some(bond) => {
            let total_delegation =
                storage::TOTAL_DELEGATION.may_load(deps.storage, bond.identity())?;
            Some(bond.attach_delegation(total_delegation.unwrap_or_default()))
        }
    };

    Ok(MixOwnershipResponse {
        address: validated_addr,
        mixnode,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::storage;
    use super::*;
    use crate::mixnodes::storage::BOND_PAGE_DEFAULT_LIMIT;
    use crate::support::tests::test_helpers;
    use crate::{contract::execute, support::tests};
    use cosmwasm_std::testing::{mock_env, mock_info};

    #[test]
    fn mixnodes_empty_on_init() {
        let deps = test_helpers::init_contract();
        let response = query_mixnodes_paged(deps.as_ref(), None, Option::from(2)).unwrap();
        assert_eq!(0, response.nodes.len());
    }

    #[test]
    fn mixnodes_paged_retrieval_obeys_limits() {
        let mut deps = test_helpers::init_contract();
        let limit = 2;
        for n in 0..1000 {
            let key = format!("bond{}", n);
            test_helpers::add_mixnode(&key, tests::fixtures::good_mixnode_pledge(), deps.as_mut());
        }

        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(limit)).unwrap();
        assert_eq!(limit, page1.nodes.len() as u32);
    }

    #[test]
    fn mixnodes_paged_retrieval_has_default_limit() {
        let mut deps = test_helpers::init_contract();
        for n in 0..1000 {
            let key = format!("bond{}", n);
            test_helpers::add_mixnode(&key, tests::fixtures::good_mixnode_pledge(), deps.as_mut());
        }

        // query without explicitly setting a limit
        let page1 = query_mixnodes_paged(deps.as_ref(), None, None).unwrap();

        assert_eq!(BOND_PAGE_DEFAULT_LIMIT, page1.nodes.len() as u32);
    }

    #[test]
    fn mixnodes_paged_retrieval_has_max_limit() {
        let mut deps = test_helpers::init_contract();
        for n in 0..1000 {
            let key = format!("bond{}", n);
            test_helpers::add_mixnode(&key, tests::fixtures::good_mixnode_pledge(), deps.as_mut());
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
        // prepare 4 messages and identities that are sorted by the generated identities
        // (because we query them in an ascended manner)
        let mut exec_data = (0..4)
            .map(|i| {
                let sender = format!("nym-addr{}", i);
                let (msg, identity) = tests::messages::valid_bond_mixnode_msg(&sender);
                (msg, (sender, identity))
            })
            .collect::<Vec<_>>();
        exec_data.sort_by(|(_, (_, id1)), (_, (_, id2))| id1.cmp(id2));
        let (messages, sender_identities): (Vec<_>, Vec<_>) = exec_data.into_iter().unzip();

        let mut deps = test_helpers::init_contract();

        let info = mock_info(
            &sender_identities[0].0.clone(),
            &tests::fixtures::good_mixnode_pledge(),
        );
        execute(deps.as_mut(), mock_env(), info, messages[0].clone()).unwrap();

        let per_page = 2;
        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();

        // page should have 1 result on it
        assert_eq!(1, page1.nodes.len());

        // save another
        let info = mock_info(
            &sender_identities[1].0.clone(),
            &tests::fixtures::good_mixnode_pledge(),
        );
        execute(deps.as_mut(), mock_env(), info, messages[1].clone()).unwrap();

        // page1 should have 2 results on it
        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.nodes.len());

        let info = mock_info(
            &sender_identities[2].0.clone(),
            &tests::fixtures::good_mixnode_pledge(),
        );
        execute(deps.as_mut(), mock_env(), info, messages[2].clone()).unwrap();

        // page1 still has 2 results
        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.nodes.len());

        // retrieving the next page should start after the last key on this page
        let start_after = page1.start_next_after.unwrap();
        let page2 = query_mixnodes_paged(
            deps.as_ref(),
            Option::from(start_after.clone()),
            Option::from(per_page),
        )
        .unwrap();

        assert_eq!(1, page2.nodes.len());

        // save another one
        let info = mock_info(
            &sender_identities[3].0.clone(),
            &tests::fixtures::good_mixnode_pledge(),
        );
        execute(deps.as_mut(), mock_env(), info, messages[3].clone()).unwrap();

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
        let res = query_owns_mixnode(deps.as_ref(), "fred".to_string()).unwrap();
        assert!(res.mixnode.is_none());

        // mixnode was added to "bob", "fred" still does not own one
        test_helpers::add_mixnode("bob", tests::fixtures::good_mixnode_pledge(), deps.as_mut());

        let res = query_owns_mixnode(deps.as_ref(), "fred".to_string()).unwrap();
        assert!(res.mixnode.is_none());

        // "fred" now owns a mixnode!
        test_helpers::add_mixnode(
            "fred",
            tests::fixtures::good_mixnode_pledge(),
            deps.as_mut(),
        );

        let res = query_owns_mixnode(deps.as_ref(), "fred".to_string()).unwrap();
        assert!(res.mixnode.is_some());

        // but after unbonding it, he doesn't own one anymore
        crate::mixnodes::transactions::try_remove_mixnode(deps.as_mut(), mock_info("fred", &[]))
            .unwrap();

        let res = query_owns_mixnode(deps.as_ref(), "fred".to_string()).unwrap();
        assert!(res.mixnode.is_none());
    }
}

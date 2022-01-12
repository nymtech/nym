// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use super::storage;
use crate::mixnodes::storage::{BOND_PAGE_DEFAULT_LIMIT, BOND_PAGE_MAX_LIMIT}; // Keeps gateway and mixnode retrieval in sync by re-using the constant. Could be split into its own constant.
use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;
use mixnet_contract_common::{
    GatewayBond, GatewayOwnershipResponse, IdentityKey, PagedGatewayResponse,
};

pub(crate) fn query_gateways_paged(
    deps: Deps,
    start_after: Option<IdentityKey>,
    limit: Option<u32>,
) -> StdResult<PagedGatewayResponse> {
    let limit = limit
        .unwrap_or(BOND_PAGE_DEFAULT_LIMIT)
        .min(BOND_PAGE_MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let nodes = storage::gateways()
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<GatewayBond>>>()?;

    let start_next_after = nodes.last().map(|node| node.identity().clone());

    Ok(PagedGatewayResponse::new(nodes, limit, start_next_after))
}

pub(crate) fn query_owns_gateway(
    deps: Deps,
    address: String,
) -> StdResult<GatewayOwnershipResponse> {
    let validated_addr = deps.api.addr_validate(&address)?;

    let gateway = storage::gateways()
        .idx
        .owner
        .item(deps.storage, validated_addr.clone())?
        .map(|record| record.1);

    Ok(GatewayOwnershipResponse {
        address: validated_addr,
        gateway,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::contract::execute;
    use crate::support::tests;
    use crate::support::tests::test_helpers;
    use cosmwasm_std::testing::{mock_env, mock_info};

    #[test]
    fn gateways_empty_on_init() {
        let deps = test_helpers::init_contract();
        let response = query_gateways_paged(deps.as_ref(), None, Option::from(2)).unwrap();
        assert_eq!(0, response.nodes.len());
    }

    #[test]
    fn gateways_paged_retrieval_obeys_limits() {
        let mut deps = test_helpers::init_contract();
        let limit = 2;
        for n in 0..1000 {
            let key = format!("bond{}", n);
            test_helpers::add_gateway(&key, tests::fixtures::good_gateway_pledge(), deps.as_mut());
        }

        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(limit)).unwrap();
        assert_eq!(limit, page1.nodes.len() as u32);
    }

    #[test]
    fn gateways_paged_retrieval_has_default_limit() {
        let mut deps = test_helpers::init_contract();
        for n in 0..1000 {
            let key = format!("bond{}", n);
            test_helpers::add_gateway(&key, tests::fixtures::good_gateway_pledge(), deps.as_mut());
        }

        // query without explicitly setting a limit
        let page1 = query_gateways_paged(deps.as_ref(), None, None).unwrap();

        assert_eq!(BOND_PAGE_DEFAULT_LIMIT, page1.nodes.len() as u32);
    }

    #[test]
    fn gateways_paged_retrieval_has_max_limit() {
        let mut deps = test_helpers::init_contract();
        for n in 0..1000 {
            let key = format!("bond{}", n);
            test_helpers::add_gateway(&key, tests::fixtures::good_gateway_pledge(), deps.as_mut());
        }

        // query with a crazily high limit in an attempt to use too many resources
        let crazy_limit = 1000 * BOND_PAGE_DEFAULT_LIMIT;
        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(crazy_limit)).unwrap();

        // we default to a decent sized upper bound instead
        let expected_limit = BOND_PAGE_MAX_LIMIT;
        assert_eq!(expected_limit, page1.nodes.len() as u32);
    }

    #[test]
    fn gateway_pagination_works() {
        // prepare 4 messages and identities that are sorted by the generated identities
        // (because we query them in an ascended manner)
        let mut exec_data = (0..4)
            .map(|i| {
                let sender = format!("nym-addr{}", i);
                let (msg, identity) = tests::messages::valid_bond_gateway_msg(&sender);
                (msg, (sender, identity))
            })
            .collect::<Vec<_>>();
        exec_data.sort_by(|(_, (_, id1)), (_, (_, id2))| id1.cmp(id2));
        let (messages, sender_identities): (Vec<_>, Vec<_>) = exec_data.into_iter().unzip();

        let mut deps = test_helpers::init_contract();

        let info = mock_info(
            &sender_identities[0].0.clone(),
            &tests::fixtures::good_gateway_pledge(),
        );
        execute(deps.as_mut(), mock_env(), info, messages[0].clone()).unwrap();

        let per_page = 2;
        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();

        // page should have 1 result on it
        assert_eq!(1, page1.nodes.len());

        // save another
        let info = mock_info(
            &sender_identities[1].0.clone(),
            &tests::fixtures::good_gateway_pledge(),
        );
        execute(deps.as_mut(), mock_env(), info, messages[1].clone()).unwrap();

        // page1 should have 2 results on it
        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.nodes.len());

        let info = mock_info(
            &sender_identities[2].0.clone(),
            &tests::fixtures::good_gateway_pledge(),
        );
        execute(deps.as_mut(), mock_env(), info, messages[2].clone()).unwrap();

        // page1 still has 2 results
        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.nodes.len());

        // retrieving the next page should start after the last key on this page
        let start_after = page1.start_next_after.unwrap();
        let page2 = query_gateways_paged(
            deps.as_ref(),
            Option::from(start_after.clone()),
            Option::from(per_page),
        )
        .unwrap();

        assert_eq!(1, page2.nodes.len());

        // save another one
        let info = mock_info(
            &sender_identities[3].0.clone(),
            &tests::fixtures::good_gateway_pledge(),
        );
        execute(deps.as_mut(), mock_env(), info, messages[3].clone()).unwrap();

        let page2 = query_gateways_paged(
            deps.as_ref(),
            Option::from(start_after),
            Option::from(per_page),
        )
        .unwrap();

        // now we have 2 pages, with 2 results on the second page
        assert_eq!(2, page2.nodes.len());
    }

    #[test]
    fn query_for_gateway_owner_works() {
        let mut deps = test_helpers::init_contract();

        // "fred" does not own a mixnode if there are no mixnodes
        let res = query_owns_gateway(deps.as_ref(), "fred".to_string()).unwrap();
        assert!(res.gateway.is_none());

        // mixnode was added to "bob", "fred" still does not own one
        test_helpers::add_gateway("bob", tests::fixtures::good_gateway_pledge(), deps.as_mut());

        let res = query_owns_gateway(deps.as_ref(), "fred".to_string()).unwrap();
        assert!(res.gateway.is_none());

        // "fred" now owns a gateway!
        test_helpers::add_gateway(
            "fred",
            tests::fixtures::good_gateway_pledge(),
            deps.as_mut(),
        );

        let res = query_owns_gateway(deps.as_ref(), "fred".to_string()).unwrap();
        assert!(res.gateway.is_some());

        // but after unbonding it, he doesn't own one anymore
        crate::gateways::transactions::try_remove_gateway(deps.as_mut(), mock_info("fred", &[]))
            .unwrap();

        let res = query_owns_gateway(deps.as_ref(), "fred".to_string()).unwrap();
        assert!(res.gateway.is_none());
    }
}

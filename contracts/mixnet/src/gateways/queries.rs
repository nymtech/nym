use super::storage;
use crate::mixnodes::storage::{BOND_PAGE_DEFAULT_LIMIT, BOND_PAGE_MAX_LIMIT}; // Keeps gateway and mixnode retrieval in sync by re-using the constant. Could be split into its own constant.
use crate::query_support::calculate_start_value;
use cosmwasm_std::{Addr, Deps, Order, StdResult};
use mixnet_contract::{GatewayBond, GatewayOwnershipResponse, IdentityKey, PagedGatewayResponse};

pub(crate) fn query_gateways_paged(
    deps: Deps,
    start_after: Option<IdentityKey>,
    limit: Option<u32>,
) -> StdResult<PagedGatewayResponse> {
    let limit = limit
        .unwrap_or(BOND_PAGE_DEFAULT_LIMIT)
        .min(BOND_PAGE_MAX_LIMIT) as usize;
    let start = calculate_start_value(start_after);

    let nodes = storage::gateways_read(deps.storage)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<GatewayBond>>>()?;

    let start_next_after = nodes.last().map(|node| node.identity().clone());

    Ok(PagedGatewayResponse::new(nodes, limit, start_next_after))
}

pub(crate) fn query_owns_gateway(deps: Deps, address: Addr) -> StdResult<GatewayOwnershipResponse> {
    let has_gateway = storage::gateways_owners_read(deps.storage)
        .may_load(address.as_bytes())?
        .is_some();
    Ok(GatewayOwnershipResponse {
        address,
        has_gateway,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    use crate::gateways::storage;
    use crate::support::tests::test_helpers;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{Addr, Storage};
    use mixnet_contract::Gateway;

    #[test]
    fn gateways_empty_on_init() {
        let deps = test_helpers::init_contract();
        let response = query_gateways_paged(deps.as_ref(), None, Option::from(2)).unwrap();
        assert_eq!(0, response.nodes.len());
    }

    fn store_n_gateway_fixtures(n: u32, storage: &mut dyn Storage) {
        for i in 0..n {
            let key = format!("bond{}", i);
            let node = test_helpers::gateway_bond_fixture();
            storage::gateways(storage)
                .save(key.as_bytes(), &node)
                .unwrap();
        }
    }

    #[test]
    fn gateways_paged_retrieval_obeys_limits() {
        let mut deps = test_helpers::init_contract();
        let storage = deps.as_mut().storage;
        let limit = 2;
        store_n_gateway_fixtures(100, storage);

        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(limit)).unwrap();
        assert_eq!(limit, page1.nodes.len() as u32);
    }

    #[test]
    fn gateways_paged_retrieval_has_default_limit() {
        let mut deps = test_helpers::init_contract();
        let storage = deps.as_mut().storage;
        store_n_gateway_fixtures(10 * BOND_PAGE_DEFAULT_LIMIT, storage);

        // query without explicitly setting a limit
        let page1 = query_gateways_paged(deps.as_ref(), None, None).unwrap();

        assert_eq!(BOND_PAGE_DEFAULT_LIMIT, page1.nodes.len() as u32);
    }

    #[test]
    fn gateways_paged_retrieval_has_max_limit() {
        let mut deps = test_helpers::init_contract();
        let storage = deps.as_mut().storage;
        store_n_gateway_fixtures(100, storage);

        // query with a crazily high limit in an attempt to use too many resources
        let crazy_limit = 1000 * BOND_PAGE_DEFAULT_LIMIT;
        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(crazy_limit)).unwrap();

        // we default to a decent sized upper bound instead
        let expected_limit = BOND_PAGE_MAX_LIMIT;
        assert_eq!(expected_limit, page1.nodes.len() as u32);
    }

    #[test]
    fn gateway_pagination_works() {
        let addr1 = "nym100";
        let addr2 = "nym101";
        let addr3 = "nym102";
        let addr4 = "nym103";

        let mut deps = test_helpers::init_contract();
        let node = test_helpers::gateway_bond_fixture();

        // TODO: note for JS when doing a deep review for the contract: add a similar test_helper as there is for mixnodes,
        // i.e. don't interact with storage here, but get the helper to call an actual transaction
        storage::gateways(&mut deps.storage)
            .save(addr1.as_bytes(), &node)
            .unwrap();

        let per_page = 2;
        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();

        // page should have 1 result on it
        assert_eq!(1, page1.nodes.len());

        // save another
        storage::gateways(&mut deps.storage)
            .save(addr2.as_bytes(), &node)
            .unwrap();

        // page1 should have 2 results on it
        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.nodes.len());

        storage::gateways(&mut deps.storage)
            .save(addr3.as_bytes(), &node)
            .unwrap();

        // page1 still has 2 results
        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.nodes.len());

        // retrieving the next page should start after the last key on this page
        let start_after = String::from(addr2);
        let page2 = query_gateways_paged(
            deps.as_ref(),
            Option::from(start_after),
            Option::from(per_page),
        )
        .unwrap();

        assert_eq!(1, page2.nodes.len());

        // save another one
        storage::gateways(&mut deps.storage)
            .save(addr4.as_bytes(), &node)
            .unwrap();

        let start_after = String::from(addr2);
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
        let res = query_owns_gateway(deps.as_ref(), Addr::unchecked("fred")).unwrap();
        assert!(!res.has_gateway);

        // mixnode was added to "bob", "fred" still does not own one
        let node = Gateway {
            identity_key: "bobsnode".into(),
            ..test_helpers::gateway_fixture()
        };
        crate::gateways::transactions::try_add_gateway(
            deps.as_mut(),
            mock_env(),
            mock_info("bob", &test_helpers::good_gateway_bond()),
            node,
        )
        .unwrap();

        let res = query_owns_gateway(deps.as_ref(), Addr::unchecked("fred")).unwrap();
        assert!(!res.has_gateway);

        // "fred" now owns a gateway!
        let node = Gateway {
            identity_key: "fredsnode".into(),
            ..test_helpers::gateway_fixture()
        };
        crate::gateways::transactions::try_add_gateway(
            deps.as_mut(),
            mock_env(),
            mock_info("fred", &test_helpers::good_gateway_bond()),
            node,
        )
        .unwrap();

        let res = query_owns_gateway(deps.as_ref(), Addr::unchecked("fred")).unwrap();
        assert!(res.has_gateway);

        // but after unbonding it, he doesn't own one anymore
        crate::gateways::transactions::try_remove_gateway(deps.as_mut(), mock_info("fred", &[]))
            .unwrap();

        let res = query_owns_gateway(deps.as_ref(), Addr::unchecked("fred")).unwrap();
        assert!(!res.has_gateway);
    }
}

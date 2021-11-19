use crate::error::ContractError;
use crate::helpers::get_all_delegations_paged;
use crate::queries::calculate_start_value;
use crate::queries::BOND_PAGE_DEFAULT_LIMIT;
use crate::storage::{
    all_mix_delegations_read, circulating_supply, config_read, gateways_owners_read, gateways_read,
    mix_delegations_read, mixnodes_owners_read, mixnodes_read, read_layer_distribution,
    read_state_params, reverse_mix_delegations_read, reward_pool_value,
};
use config::defaults::DENOM;
use cosmwasm_std::{coin, Addr, Deps, Order, StdResult, Uint128};
use mixnet_contract::{
    Delegation, GatewayBond, GatewayOwnershipResponse, IdentityKey, LayerDistribution, MixNodeBond,
    MixOwnershipResponse, PagedAllDelegationsResponse, PagedGatewayResponse,
    PagedMixDelegationsResponse, PagedMixnodeResponse, PagedReverseMixDelegationsResponse,
    RawDelegationData, RewardingIntervalResponse, StateParams,
};

pub(crate) fn query_gateways_paged(
    deps: Deps,
    start_after: Option<IdentityKey>,
    limit: Option<u32>,
) -> StdResult<PagedGatewayResponse> {
    let limit = limit
        .unwrap_or(BOND_PAGE_DEFAULT_LIMIT)
        .min(BOND_PAGE_DEFAULT_LIMIT) as usize;
    let start = calculate_start_value(start_after);

    let nodes = gateways_read(deps.storage)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<GatewayBond>>>()?;

    let start_next_after = nodes.last().map(|node| node.identity().clone());

    Ok(PagedGatewayResponse::new(nodes, limit, start_next_after))
}

pub(crate) fn query_owns_gateway(deps: Deps, address: Addr) -> StdResult<GatewayOwnershipResponse> {
    let has_gateway = gateways_owners_read(deps.storage)
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
    use crate::mixnet_params::state::State;
    use crate::storage::{config, gateways, mix_delegations, mixnodes};
    use crate::support::tests::helpers;
    use crate::support::tests::helpers::{
        good_gateway_bond, good_mixnode_bond, raw_delegation_fixture,
    };
    use cosmwasm_std::testing::{mock_env, mock_info};
    use cosmwasm_std::{Addr, Storage};
    use mixnet_contract::{Gateway, MixNode, RawDelegationData};

    #[test]
    fn gateways_empty_on_init() {
        let deps = helpers::init_contract();
        let response = query_gateways_paged(deps.as_ref(), None, Option::from(2)).unwrap();
        assert_eq!(0, response.nodes.len());
    }

    fn store_n_gateway_fixtures(n: u32, storage: &mut dyn Storage) {
        for i in 0..n {
            let key = format!("bond{}", i);
            let node = helpers::gateway_bond_fixture();
            gateways(storage).save(key.as_bytes(), &node).unwrap();
        }
    }

    #[test]
    fn gateways_paged_retrieval_obeys_limits() {
        let mut deps = helpers::init_contract();
        let storage = deps.as_mut().storage;
        let limit = 2;
        store_n_gateway_fixtures(100, storage);

        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(limit)).unwrap();
        assert_eq!(limit, page1.nodes.len() as u32);
    }

    #[test]
    fn gateways_paged_retrieval_has_default_limit() {
        let mut deps = helpers::init_contract();
        let storage = deps.as_mut().storage;
        store_n_gateway_fixtures(10 * BOND_PAGE_DEFAULT_LIMIT, storage);

        // query without explicitly setting a limit
        let page1 = query_gateways_paged(deps.as_ref(), None, None).unwrap();

        assert_eq!(BOND_PAGE_DEFAULT_LIMIT, page1.nodes.len() as u32);
    }

    #[test]
    fn gateways_paged_retrieval_has_max_limit() {
        let mut deps = helpers::init_contract();
        let storage = deps.as_mut().storage;
        store_n_gateway_fixtures(100, storage);

        // query with a crazily high limit in an attempt to use too many resources
        let crazy_limit = 1000 * BOND_PAGE_DEFAULT_LIMIT;
        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(crazy_limit)).unwrap();

        // we default to a decent sized upper bound instead
        let expected_limit = BOND_PAGE_DEFAULT_LIMIT;
        assert_eq!(expected_limit, page1.nodes.len() as u32);
    }

    #[test]
    fn gateway_pagination_works() {
        let addr1 = "hal100";
        let addr2 = "hal101";
        let addr3 = "hal102";
        let addr4 = "hal103";

        let mut deps = helpers::init_contract();
        let node = helpers::gateway_bond_fixture();
        gateways(&mut deps.storage)
            .save(addr1.as_bytes(), &node)
            .unwrap();

        let per_page = 2;
        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();

        // page should have 1 result on it
        assert_eq!(1, page1.nodes.len());

        // save another
        gateways(&mut deps.storage)
            .save(addr2.as_bytes(), &node)
            .unwrap();

        // page1 should have 2 results on it
        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.nodes.len());

        gateways(&mut deps.storage)
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
        gateways(&mut deps.storage)
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
        let mut deps = helpers::init_contract();

        // "fred" does not own a mixnode if there are no mixnodes
        let res = query_owns_gateway(deps.as_ref(), Addr::unchecked("fred")).unwrap();
        assert!(!res.has_gateway);

        // mixnode was added to "bob", "fred" still does not own one
        let node = Gateway {
            identity_key: "bobsnode".into(),
            ..helpers::gateway_fixture()
        };
        crate::gateways::transactions::try_add_gateway(
            deps.as_mut(),
            mock_env(),
            mock_info("bob", &good_gateway_bond()),
            node,
        )
        .unwrap();

        let res = query_owns_gateway(deps.as_ref(), Addr::unchecked("fred")).unwrap();
        assert!(!res.has_gateway);

        // "fred" now owns a gateway!
        let node = Gateway {
            identity_key: "fredsnode".into(),
            ..helpers::gateway_fixture()
        };
        crate::gateways::transactions::try_add_gateway(
            deps.as_mut(),
            mock_env(),
            mock_info("fred", &good_gateway_bond()),
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

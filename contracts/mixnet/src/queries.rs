use crate::state::StateParams;
use crate::storage::{gateways_read, mixnodes_read, read_state_params};
use cosmwasm_std::Deps;
use cosmwasm_std::HumanAddr;
use cosmwasm_std::Order;
use cosmwasm_std::StdResult;
use mixnet_contract::{
    GatewayBond, GatewayOwnershipResponse, MixNodeBond, MixOwnershipResponse, PagedGatewayResponse,
    PagedResponse,
};

const MAX_LIMIT: u32 = 100;
const DEFAULT_LIMIT: u32 = 50;

pub fn query_mixnodes_paged(
    deps: Deps,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<PagedResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calculate_start_value(start_after);

    let nodes = mixnodes_read(deps.storage)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<MixNodeBond>>>()?;

    let start_next_after = nodes.last().map(|node| node.owner().clone());

    Ok(PagedResponse::new(nodes, limit, start_next_after))
}

pub(crate) fn query_gateways_paged(
    deps: Deps,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<PagedGatewayResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calculate_start_value(start_after);

    let nodes = gateways_read(deps.storage)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<GatewayBond>>>()?;

    let start_next_after = nodes.last().map(|node| node.owner().clone());

    Ok(PagedGatewayResponse::new(nodes, limit, start_next_after))
}

pub(crate) fn query_owns_mixnode(
    deps: Deps,
    address: HumanAddr,
) -> StdResult<MixOwnershipResponse> {
    let has_node = mixnodes_read(deps.storage)
        .may_load(address.as_ref())?
        .is_some();
    Ok(MixOwnershipResponse { address, has_node })
}

pub(crate) fn query_owns_gateway(
    deps: Deps,
    address: HumanAddr,
) -> StdResult<GatewayOwnershipResponse> {
    let has_gateway = gateways_read(deps.storage)
        .may_load(address.as_ref())?
        .is_some();
    Ok(GatewayOwnershipResponse {
        address,
        has_gateway,
    })
}

pub(crate) fn query_state_params(deps: Deps) -> StateParams {
    read_state_params(deps.storage)
}

/// Adds a 0 byte to terminate the `start_after` value given. This allows CosmWasm
/// to get the succeeding key as the start of the next page.
fn calculate_start_value(
    start_after: std::option::Option<cosmwasm_std::HumanAddr>,
) -> Option<Vec<u8>> {
    start_after.as_ref().map(|addr| {
        let mut bytes = addr.as_bytes().to_owned();
        bytes.push(0);
        bytes
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::State;
    use crate::storage::{config, gateways, mixnodes};
    use crate::support::tests::helpers;
    use cosmwasm_std::Storage;

    #[test]
    fn mixnodes_empty_on_init() {
        let deps = helpers::init_contract();
        let response = query_mixnodes_paged(deps.as_ref(), None, Option::from(2)).unwrap();
        assert_eq!(0, response.nodes.len());
    }

    #[test]
    fn mixnodes_paged_retrieval_obeys_limits() {
        let mut deps = helpers::init_contract();
        let storage = deps.as_mut().storage;
        let limit = 2;
        for n in 0..10000 {
            let key = format!("bond{}", n);
            let node = helpers::mixnode_bond_fixture();
            mixnodes(storage).save(key.as_bytes(), &node).unwrap();
        }

        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(limit)).unwrap();
        assert_eq!(limit, page1.nodes.len() as u32);
    }

    #[test]
    fn mixnodes_paged_retrieval_has_default_limit() {
        let mut deps = helpers::init_contract();
        let storage = deps.as_mut().storage;
        for n in 0..100 {
            let key = format!("bond{}", n);
            let node = helpers::mixnode_bond_fixture();
            mixnodes(storage).save(key.as_bytes(), &node).unwrap();
        }

        // query without explicitly setting a limit
        let page1 = query_mixnodes_paged(deps.as_ref(), None, None).unwrap();

        let expected_limit = 50;
        assert_eq!(expected_limit, page1.nodes.len() as u32);
    }

    #[test]
    fn mixnodes_paged_retrieval_has_max_limit() {
        let mut deps = helpers::init_contract();
        let storage = deps.as_mut().storage;
        for n in 0..10000 {
            let key = format!("bond{}", n);
            let node = helpers::mixnode_bond_fixture();
            mixnodes(storage).save(key.as_bytes(), &node).unwrap();
        }

        // query with a crazily high limit in an attempt to use too many resources
        let crazy_limit = 1000;
        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(crazy_limit)).unwrap();

        // we default to a decent sized upper bound instead
        let expected_limit = 100;
        assert_eq!(expected_limit, page1.nodes.len() as u32);
    }

    #[test]
    fn pagination_works() {
        let mut deps = helpers::init_contract();
        let node = helpers::mixnode_bond_fixture();
        mixnodes(&mut deps.storage)
            .save("1".as_bytes(), &node)
            .unwrap();

        let per_page = 2;
        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();

        // page should have 1 result on it
        assert_eq!(1, page1.nodes.len());

        // save another
        mixnodes(&mut deps.storage)
            .save("2".as_bytes(), &node)
            .unwrap();

        // page1 should have 2 results on it
        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.nodes.len());

        mixnodes(&mut deps.storage)
            .save("3".as_bytes(), &node)
            .unwrap();

        // page1 still has 2 results
        let page1 = query_mixnodes_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.nodes.len());

        // retrieving the next page should start after the last key on this page
        let start_after = HumanAddr::from("2");
        let page2 = query_mixnodes_paged(
            deps.as_ref(),
            Option::from(start_after),
            Option::from(per_page),
        )
        .unwrap();

        assert_eq!(1, page2.nodes.len());

        // save another one
        mixnodes(&mut deps.storage)
            .save("4".as_bytes(), &node)
            .unwrap();

        let start_after = HumanAddr::from("2");
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
        store_n_gateway_fixtures(10 * DEFAULT_LIMIT, storage);

        // query without explicitly setting a limit
        let page1 = query_gateways_paged(deps.as_ref(), None, None).unwrap();

        assert_eq!(DEFAULT_LIMIT, page1.nodes.len() as u32);
    }

    #[test]
    fn gateways_paged_retrieval_has_max_limit() {
        let mut deps = helpers::init_contract();
        let storage = deps.as_mut().storage;
        store_n_gateway_fixtures(100, storage);

        // query with a crazily high limit in an attempt to use too many resources
        let crazy_limit = 1000 * DEFAULT_LIMIT;
        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(crazy_limit)).unwrap();

        // we default to a decent sized upper bound instead
        let expected_limit = MAX_LIMIT;
        assert_eq!(expected_limit, page1.nodes.len() as u32);
    }

    #[test]
    fn gateway_pagination_works() {
        let mut deps = helpers::init_contract();
        let node = helpers::gateway_bond_fixture();
        gateways(&mut deps.storage)
            .save("1".as_bytes(), &node)
            .unwrap();

        let per_page = 2;
        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();

        // page should have 1 result on it
        assert_eq!(1, page1.nodes.len());

        // save another
        gateways(&mut deps.storage)
            .save("2".as_bytes(), &node)
            .unwrap();

        // page1 should have 2 results on it
        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.nodes.len());

        gateways(&mut deps.storage)
            .save("3".as_bytes(), &node)
            .unwrap();

        // page1 still has 2 results
        let page1 = query_gateways_paged(deps.as_ref(), None, Option::from(per_page)).unwrap();
        assert_eq!(2, page1.nodes.len());

        // retrieving the next page should start after the last key on this page
        let start_after = HumanAddr::from("2");
        let page2 = query_gateways_paged(
            deps.as_ref(),
            Option::from(start_after),
            Option::from(per_page),
        )
        .unwrap();

        assert_eq!(1, page2.nodes.len());

        // save another one
        gateways(&mut deps.storage)
            .save("4".as_bytes(), &node)
            .unwrap();

        let start_after = HumanAddr::from("2");
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
    fn query_for_mixnode_owner_works() {
        let mut deps = helpers::init_contract();

        // "fred" does not own a mixnode if there are no mixnodes
        let res = query_owns_mixnode(deps.as_ref(), "fred".into()).unwrap();
        assert!(!res.has_node);

        // mixnode was added to "bob", "fred" still does not own one
        let node = helpers::mixnode_bond_fixture();
        mixnodes(&mut deps.storage)
            .save("bob".as_bytes(), &node)
            .unwrap();

        let res = query_owns_mixnode(deps.as_ref(), "fred".into()).unwrap();
        assert!(!res.has_node);

        // "fred" now owns a mixnode!
        let node2 = helpers::mixnode_bond_fixture();
        mixnodes(&mut deps.storage)
            .save("fred".as_bytes(), &node2)
            .unwrap();

        let res = query_owns_mixnode(deps.as_ref(), "fred".into()).unwrap();
        assert!(res.has_node);

        // but after unbonding it, he doesn't own one anymore
        mixnodes(&mut deps.storage).remove("fred".as_bytes());
        let res = query_owns_mixnode(deps.as_ref(), "fred".into()).unwrap();
        assert!(!res.has_node);
    }

    #[test]
    fn query_for_gateway_owner_works() {
        let mut deps = helpers::init_contract();

        // "fred" does not own a mixnode if there are no mixnodes
        let res = query_owns_gateway(deps.as_ref(), "fred".into()).unwrap();
        assert!(!res.has_gateway);

        // mixnode was added to "bob", "fred" still does not own one
        let node = helpers::gateway_bond_fixture();
        gateways(&mut deps.storage)
            .save("bob".as_bytes(), &node)
            .unwrap();

        let res = query_owns_gateway(deps.as_ref(), "fred".into()).unwrap();
        assert!(!res.has_gateway);

        // "fred" now owns a mixnode!
        let node2 = helpers::gateway_bond_fixture();
        gateways(&mut deps.storage)
            .save("fred".as_bytes(), &node2)
            .unwrap();

        let res = query_owns_gateway(deps.as_ref(), "fred".into()).unwrap();
        assert!(res.has_gateway);

        // but after unbonding it, he doesn't own one anymore
        gateways(&mut deps.storage).remove("fred".as_bytes());
        let res = query_owns_gateway(deps.as_ref(), "fred".into()).unwrap();
        assert!(!res.has_gateway);
    }

    #[test]
    fn query_for_contract_state_works() {
        let mut deps = helpers::init_contract();

        let dummy_state = State {
            owner: "someowner".into(),
            network_monitor_address: "monitor".into(),
            params: StateParams {
                epoch_length: 1,
                minimum_mixnode_bond: 123u128.into(),
                minimum_gateway_bond: 456u128.into(),
                mixnode_bond_reward_rate: "1.23".parse().unwrap(),
                gateway_bond_reward_rate: "4.56".parse().unwrap(),
                mixnode_active_set_size: 1000,
            },
            mixnode_epoch_bond_reward: "1.23".parse().unwrap(),
            gateway_epoch_bond_reward: "4.56".parse().unwrap(),
        };

        config(deps.as_mut().storage).save(&dummy_state).unwrap();

        assert_eq!(dummy_state.params, query_state_params(deps.as_ref()))
    }
}

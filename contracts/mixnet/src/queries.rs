// settings for pagination
use crate::state::{gateways_read, PREFIX_MIXNODES};
use cosmwasm_std::Deps;
use cosmwasm_std::HumanAddr;
use cosmwasm_std::Order;
use cosmwasm_std::StdResult;
use cosmwasm_storage::bucket_read;
use mixnet_contract::{GatewayBond, MixNodeBond};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct PagedResponse {
    pub nodes: Vec<MixNodeBond>,
    pub per_page: usize,
    pub start_next_after: Option<HumanAddr>,
}

pub fn query_mixnodes_paged(
    deps: Deps,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<PagedResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calculate_start_value(start_after);

    let bucket = bucket_read::<MixNodeBond>(deps.storage, PREFIX_MIXNODES);
    let res = bucket
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit);
    let node_tuples = res.collect::<StdResult<Vec<(Vec<u8>, MixNodeBond)>>>()?;
    let nodes = node_tuples
        .into_iter()
        .map(|item| item.1)
        .collect::<Vec<_>>();
    let start_next_after = last_node_owner(&nodes);

    let response = PagedResponse {
        nodes,
        per_page: limit,
        start_next_after,
    };
    Ok(response)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub struct PagedGatewayResponse {
    pub(crate) nodes: Vec<GatewayBond>,
    pub(crate) per_page: usize,
    pub(crate) start_next_after: Option<HumanAddr>,
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

    Ok(PagedGatewayResponse {
        nodes,
        per_page: limit,
        start_next_after,
    })
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

fn last_node_owner(nodes: &[MixNodeBond]) -> Option<HumanAddr> {
    match nodes.last() {
        None => None,
        Some(node) => Some(node.owner().clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{gateways, mixnodes};
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

        let expected_limit = 10;
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
        let expected_limit = 30;
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
}

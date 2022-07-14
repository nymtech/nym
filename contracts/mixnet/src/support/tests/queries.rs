use cosmwasm_std::{
    from_binary,
    testing::{mock_env, MockApi, MockQuerier, MockStorage},
    Addr, Coin, OwnedDeps,
};
use mixnet_contract_common::mixnode::{MixNodeDetails, PagedMixnodesDetailsResponse};
use mixnet_contract_common::{GatewayBond, PagedGatewayResponse, QueryMsg};

use crate::contract::query;
use crate::support::tests::fixtures::TEST_COIN_DENOM;

pub fn get_mix_nodes(
    deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>,
) -> Vec<MixNodeDetails> {
    let result = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetMixNodesDetailed {
            start_after: None,
            limit: None,
        },
    )
    .unwrap();

    let page: PagedMixnodesDetailsResponse = from_binary(&result).unwrap();
    if page.start_next_after.is_some() {
        panic!("we didn't manage to get all mixnodes in a single query")
    }
    page.nodes
}

pub fn get_gateways(deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>) -> Vec<GatewayBond> {
    let result = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetGateways {
            start_after: None,
            limit: None,
        },
    )
    .unwrap();

    let page: PagedGatewayResponse = from_binary(&result).unwrap();
    if page.start_next_after.is_some() {
        panic!("we didn't manage to get all gateways in a single query")
    }
    page.nodes
}

pub fn query_contract_balance(
    address: Addr,
    deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
) -> Vec<Coin> {
    let querier = deps.as_ref().querier;
    vec![querier.query_balance(address, TEST_COIN_DENOM).unwrap()]
}

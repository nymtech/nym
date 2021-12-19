use config::defaults::DENOM;
use cosmwasm_std::{
    from_binary,
    testing::{mock_env, MockApi, MockQuerier, MockStorage},
    Addr, Coin, OwnedDeps,
};
use mixnet_contract::{
    GatewayBond, MixNodeBond, PagedGatewayResponse, PagedMixnodeResponse, QueryMsg,
};

use crate::contract::query;

pub fn get_mix_nodes(deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>) -> Vec<MixNodeBond> {
    let result = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetMixNodes {
            start_after: None,
            limit: Option::from(2),
        },
    )
    .unwrap();

    let page: PagedMixnodeResponse = from_binary(&result).unwrap();
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
    page.nodes
}

pub fn query_contract_balance(
    address: Addr,
    deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
) -> Vec<Coin> {
    let querier = deps.as_ref().querier;
    vec![querier.query_balance(address, DENOM).unwrap()]
}

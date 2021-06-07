#[cfg(test)]
pub mod helpers {
    use super::*;
    use crate::contract::init;
    use crate::contract::query;
    use crate::contract::DENOM;
    use crate::msg::InitMsg;
    use crate::msg::QueryMsg;
    use crate::transactions::{try_add_gateway, try_add_mixnode};
    use cosmwasm_std::coins;
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::testing::MockApi;
    use cosmwasm_std::testing::MockQuerier;
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::Coin;
    use cosmwasm_std::HumanAddr;
    use cosmwasm_std::OwnedDeps;
    use cosmwasm_std::{Empty, MemoryStorage};
    use mixnet_contract::{
        Gateway, GatewayBond, MixNode, MixNodeBond, PagedGatewayResponse, PagedResponse,
    };

    pub fn add_mixnode(
        pubkey: &str,
        stake: Vec<Coin>,
        deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ) {
        let info = mock_info(pubkey, &stake);
        try_add_mixnode(
            deps.as_mut(),
            info,
            MixNode {
                identity_key: format!("{}mixnode", pubkey),
                ..helpers::mix_node_fixture()
            },
        )
        .unwrap();
    }

    pub fn get_mix_nodes(
        deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ) -> Vec<MixNodeBond> {
        let result = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetMixNodes {
                start_after: None,
                limit: Option::from(2),
            },
        )
        .unwrap();

        let page: PagedResponse = from_binary(&result).unwrap();
        page.nodes
    }

    pub fn add_gateway(
        pubkey: &str,
        stake: Vec<Coin>,
        deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ) {
        let info = mock_info(pubkey, &stake);
        try_add_gateway(
            deps.as_mut(),
            info,
            Gateway {
                identity_key: format!("{}gateway", pubkey),
                ..helpers::gateway_fixture()
            },
        )
        .unwrap();
    }

    pub fn get_gateways(
        deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ) -> Vec<GatewayBond> {
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

    pub fn init_contract() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
        let mut deps = mock_dependencies(&[]);
        let msg = InitMsg {};
        let env = mock_env();
        let info = mock_info("creator", &[]);
        init(deps.as_mut(), env.clone(), info, msg).unwrap();
        return deps;
    }

    pub fn mix_node_fixture() -> MixNode {
        MixNode::new(
            "mix.node.org".to_string(),
            1,
            "Sweden".to_string(),
            "sphinx".to_string(),
            "identity".to_string(),
            "0.10.0".to_string(),
        )
    }

    pub fn mixnode_bond_fixture() -> MixNodeBond {
        let mix_node = MixNode::new(
            "1.1.1.1".to_string(),
            1,
            "London".to_string(),
            "1234".to_string(),
            "aaaa".to_string(),
            "0.10.0".to_string(),
        );
        MixNodeBond::new(coins(50, DENOM), HumanAddr::from("foo"), mix_node)
    }

    pub fn gateway_fixture() -> Gateway {
        Gateway::new(
            "1.1.1.1:1234".to_string(),
            "ws://1.1.1.1:1235".to_string(),
            "Sweden".to_string(),
            "sphinx".to_string(),
            "identity".to_string(),
            "0.10.0".to_string(),
        )
    }

    pub fn gateway_bond_fixture() -> GatewayBond {
        let gateway = Gateway::new(
            "1.1.1.1:1234".to_string(),
            "ws://1.1.1.1:1235".to_string(),
            "London".to_string(),
            "sphinx".to_string(),
            "identity".to_string(),
            "0.10.0".to_string(),
        );
        GatewayBond::new(coins(50, DENOM), HumanAddr::from("foo"), gateway)
    }

    pub fn query_contract_balance(
        address: HumanAddr,
        deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ) -> Vec<Coin> {
        let querier = deps.as_ref().querier;
        vec![querier.query_balance(address, DENOM).unwrap()]
    }
}

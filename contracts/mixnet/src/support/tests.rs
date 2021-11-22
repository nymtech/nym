#[cfg(test)]
pub mod helpers {
    use super::*;
    use crate::contract::query;
    use crate::contract::{instantiate, INITIAL_MIXNODE_BOND};
    use crate::transactions::{try_add_gateway, try_add_mixnode};
    use crate::StoredMixnodeBond;
    use config::defaults::DENOM;
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::testing::MockApi;
    use cosmwasm_std::testing::MockQuerier;
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::Addr;
    use cosmwasm_std::Coin;
    use cosmwasm_std::OwnedDeps;
    use cosmwasm_std::{coin, Uint128};
    use cosmwasm_std::{from_binary, DepsMut};
    use cosmwasm_std::{Empty, MemoryStorage};
    use mixnet_contract::{
        Gateway, GatewayBond, InstantiateMsg, Layer, MixNode, MixNodeBond, PagedGatewayResponse,
        PagedMixnodeResponse, QueryMsg, RawDelegationData,
    };

    pub fn add_mixnode(sender: &str, stake: Vec<Coin>, deps: DepsMut) -> String {
        let info = mock_info(sender, &stake);
        let key = format!("{}mixnode", sender);
        try_add_mixnode(
            deps,
            mock_env(),
            info,
            MixNode {
                identity_key: key.clone(),
                ..helpers::mix_node_fixture()
            },
        )
        .unwrap();
        key
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

        let page: PagedMixnodeResponse = from_binary(&result).unwrap();
        page.nodes
    }

    pub fn add_gateway(
        sender: &str,
        stake: Vec<Coin>,
        deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ) -> String {
        let info = mock_info(sender, &stake);
        let key = format!("{}gateway", sender);
        try_add_gateway(
            deps.as_mut(),
            mock_env(),
            info,
            Gateway {
                identity_key: key.clone(),
                ..helpers::gateway_fixture()
            },
        )
        .unwrap();
        key
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
        let msg = InstantiateMsg {};
        let env = mock_env();
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        return deps;
    }

    pub fn mix_node_fixture() -> MixNode {
        MixNode {
            host: "mix.node.org".to_string(),
            mix_port: 1789,
            verloc_port: 1790,
            http_api_port: 8000,
            sphinx_key: "sphinx".to_string(),
            identity_key: "identity".to_string(),
            version: "0.10.0".to_string(),
        }
    }

    pub fn mixnode_bond_fixture() -> MixNodeBond {
        let mix_node = MixNode {
            host: "1.1.1.1".to_string(),
            mix_port: 1789,
            verloc_port: 1790,
            http_api_port: 8000,
            sphinx_key: "1234".to_string(),
            identity_key: "aaaa".to_string(),
            version: "0.10.0".to_string(),
        };
        MixNodeBond::new(
            coin(50, DENOM),
            Addr::unchecked("foo"),
            Layer::One,
            12_345,
            mix_node,
            None,
        )
    }

    pub(crate) fn stored_mixnode_bond_fixture() -> StoredMixnodeBond {
        StoredMixnodeBond::new(
            coin(50, DENOM),
            Addr::unchecked("foo"),
            Layer::One,
            12_345,
            mix_node_fixture(),
            None,
        )
    }

    pub fn gateway_fixture() -> Gateway {
        Gateway {
            host: "1.1.1.1".to_string(),
            mix_port: 1789,
            clients_port: 9000,
            location: "Sweden".to_string(),

            sphinx_key: "sphinx".to_string(),
            identity_key: "identity".to_string(),
            version: "0.10.0".to_string(),
        }
    }

    pub fn gateway_bond_fixture() -> GatewayBond {
        let gateway = Gateway {
            host: "1.1.1.1".to_string(),
            mix_port: 1789,
            clients_port: 9000,
            location: "London".to_string(),
            sphinx_key: "sphinx".to_string(),
            identity_key: "identity".to_string(),
            version: "0.10.0".to_string(),
        };
        GatewayBond::new(coin(50, DENOM), Addr::unchecked("foo"), 12_345, gateway)
    }

    pub fn raw_delegation_fixture(amount: u128) -> RawDelegationData {
        RawDelegationData::new(Uint128(amount), 42)
    }

    pub fn query_contract_balance(
        address: Addr,
        deps: OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ) -> Vec<Coin> {
        let querier = deps.as_ref().querier;
        vec![querier.query_balance(address, DENOM).unwrap()]
    }

    pub fn good_mixnode_bond() -> Vec<Coin> {
        vec![Coin {
            denom: DENOM.to_string(),
            amount: INITIAL_MIXNODE_BOND,
        }]
    }

    pub fn good_gateway_bond() -> Vec<Coin> {
        vec![Coin {
            denom: DENOM.to_string(),
            amount: INITIAL_MIXNODE_BOND,
        }]
    }
}

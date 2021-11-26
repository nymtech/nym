#[cfg(test)]
pub mod test_helpers {
    use super::*;
    use crate::contract::{instantiate, INITIAL_MIXNODE_BOND};
    use crate::contract::{
        query, DEFAULT_SYBIL_RESISTANCE_PERCENT, EPOCH_REWARD_PERCENT, INITIAL_REWARD_POOL,
    };
    use crate::gateways::transactions::try_add_gateway;
    use crate::mixnodes::storage as mixnodes_storage;
    use crate::mixnodes::storage::StoredMixnodeBond;
    use crate::mixnodes::transactions::try_add_mixnode;
    use config::defaults::{DENOM, TOTAL_SUPPLY};
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::testing::MockApi;
    use cosmwasm_std::testing::MockQuerier;
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::Coin;
    use cosmwasm_std::OwnedDeps;
    use cosmwasm_std::{coin, Uint128};
    use cosmwasm_std::{from_binary, DepsMut};
    use cosmwasm_std::{Addr, StdResult, Storage};
    use cosmwasm_std::{Empty, MemoryStorage};
    use mixnet_contract::mixnode::NodeRewardParams;
    use mixnet_contract::{
        Gateway, GatewayBond, IdentityKeyRef, InstantiateMsg, Layer, MixNode, MixNodeBond,
        PagedGatewayResponse, PagedMixnodeResponse, QueryMsg, RawDelegationData,
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
                ..test_helpers::mix_node_fixture()
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

    pub fn add_gateway(sender: &str, stake: Vec<Coin>, deps: DepsMut) -> String {
        let info = mock_info(sender, &stake);
        let key = format!("{}gateway", sender);
        try_add_gateway(
            deps,
            mock_env(),
            info,
            Gateway {
                identity_key: key.clone(),
                ..test_helpers::gateway_fixture()
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
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {};
        let env = mock_env();
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        deps
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

    pub(crate) fn stored_mixnode_bond_fixture(owner: &str) -> mixnodes_storage::StoredMixnodeBond {
        StoredMixnodeBond::new(
            coin(50, DENOM),
            Addr::unchecked(owner),
            Layer::One,
            12_345,
            MixNode {
                identity_key: format!("id-{}", owner),
                ..mix_node_fixture()
            },
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

    pub fn gateway_bond_fixture(owner: &str) -> GatewayBond {
        let gateway = Gateway {
            identity_key: format!("id-{}", owner),
            ..gateway_fixture()
        };
        GatewayBond::new(coin(50, DENOM), Addr::unchecked(owner), 12_345, gateway)
    }

    pub fn raw_delegation_fixture(amount: u128) -> RawDelegationData {
        RawDelegationData::new(Uint128::new(amount), 42)
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

    // when exact values are irrelevant and what matters is the action of rewarding
    pub fn node_rewarding_params_fixture(uptime: u128) -> NodeRewardParams {
        NodeRewardParams::new(
            (INITIAL_REWARD_POOL / 100) * EPOCH_REWARD_PERCENT as u128,
            50 as u128,
            0,
            TOTAL_SUPPLY - INITIAL_REWARD_POOL,
            uptime,
            DEFAULT_SYBIL_RESISTANCE_PERCENT,
        )
    }

    // Converts the node identity and owner of a delegation into the bytes used as
    // key in the delegation buckets. Basically a helper function.
    pub(crate) fn identity_and_owner_to_bytes(identity: &str, owner: &Addr) -> Vec<u8> {
        let mut bytes = u16::to_be_bytes(identity.len() as u16).to_vec();
        bytes.append(&mut identity.as_bytes().to_vec());
        bytes.append(&mut owner.as_bytes().to_vec());

        bytes
    }

    // currently not used outside tests
    pub(crate) fn read_mixnode_bond_amount(
        storage: &dyn Storage,
        identity: IdentityKeyRef,
    ) -> StdResult<cosmwasm_std::Uint128> {
        let node = mixnodes_storage::mixnodes().load(storage, identity)?;
        Ok(node.bond_amount.amount)
    }
}

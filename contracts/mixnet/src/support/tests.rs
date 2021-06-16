#[cfg(test)]
pub mod helpers {
    use super::*;
    use crate::contract::query;
    use crate::contract::DENOM;
    use crate::contract::{init, INITIAL_MIXNODE_BOND};
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
        EncryptionStringPublicKeyWrapper, Gateway, GatewayBond, IdentityStringPublicKeyWrapper,
        MixNode, MixNodeBond, PagedGatewayResponse, PagedResponse,
    };
    use sha3::Digest;
    use std::ops::Deref;

    // helper trait to allow easy test identities construction, like `"foo".hash_to_identity()`;
    pub(crate) trait HashToIdentity {
        fn hash_to_identity(self) -> IdentityStringPublicKeyWrapper;
    }

    impl<I> HashToIdentity for I
    where
        I: AsRef<[u8]>,
    {
        // just hash and increment
        // this is a very naive way of getting identity out of an arbitrary string
        // but considering it's only used for tests, it's good enough
        fn hash_to_identity(self) -> IdentityStringPublicKeyWrapper {
            let mut h = sha3::Sha3_256::new();

            let mut ctr = 0u64;
            loop {
                h.update(self.as_ref());
                h.update(&ctr.to_le_bytes());
                ctr += 1;

                let digest = h.finalize_reset();

                let array: [u8; 32] = digest.into();
                if let Ok(key) =
                    <IdentityStringPublicKeyWrapper as Deref>::Target::from_bytes(&array)
                {
                    return IdentityStringPublicKeyWrapper(key);
                }
            }
        }
    }

    // this one is only ever used to create fixtures here so it doesn't need a full-blown trait definition
    pub(crate) fn hash_to_sphinx_key<I: AsRef<[u8]>>(val: I) -> EncryptionStringPublicKeyWrapper {
        let mut h = sha3::Sha3_256::new();

        let mut ctr = 0u64;
        loop {
            h.update(val.as_ref());
            h.update(&ctr.to_le_bytes());
            ctr += 1;

            let digest = h.finalize_reset();

            let array: [u8; 32] = digest.into();
            if let Ok(key) = <EncryptionStringPublicKeyWrapper as Deref>::Target::from_bytes(&array)
            {
                return EncryptionStringPublicKeyWrapper(key);
            }
        }
    }

    pub fn add_mixnode(
        sender: &str,
        stake: Vec<Coin>,
        deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ) -> IdentityStringPublicKeyWrapper {
        let info = mock_info(sender, &stake);
        let key = format!("{}mixnode", sender).hash_to_identity();
        try_add_mixnode(
            deps.as_mut(),
            info,
            MixNode {
                identity_key: key,
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

        let page: PagedResponse = from_binary(&result).unwrap();
        page.nodes
    }

    pub fn add_gateway(
        sender: &str,
        stake: Vec<Coin>,
        deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier>,
    ) -> IdentityStringPublicKeyWrapper {
        let info = mock_info(sender, &stake);
        let key = format!("{}gateway", sender).hash_to_identity();
        try_add_gateway(
            deps.as_mut(),
            info,
            Gateway {
                identity_key: key,
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
            hash_to_sphinx_key("sphinx"),
            "identity".hash_to_identity(),
            "0.10.0".to_string(),
        )
    }

    pub fn mixnode_bond_fixture() -> MixNodeBond {
        let mix_node = MixNode::new(
            "1.1.1.1".to_string(),
            1,
            "London".to_string(),
            hash_to_sphinx_key("1234"),
            "aaaa".hash_to_identity(),
            "0.10.0".to_string(),
        );
        MixNodeBond::new(coins(50, DENOM), HumanAddr::from("foo"), mix_node)
    }

    pub fn gateway_fixture() -> Gateway {
        Gateway::new(
            "1.1.1.1:1234".to_string(),
            "ws://1.1.1.1:1235".to_string(),
            "Sweden".to_string(),
            hash_to_sphinx_key("sphinx"),
            "identity".hash_to_identity(),
            "0.10.0".to_string(),
        )
    }

    pub fn gateway_bond_fixture() -> GatewayBond {
        let gateway = Gateway::new(
            "1.1.1.1:1234".to_string(),
            "ws://1.1.1.1:1235".to_string(),
            "London".to_string(),
            hash_to_sphinx_key("sphinx"),
            "identity".hash_to_identity(),
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

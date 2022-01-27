// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
#[cfg(test)]
pub mod fixtures;
#[cfg(test)]
pub mod messages;
#[cfg(test)]
pub mod queries;

#[cfg(test)]
pub mod test_helpers {
    use crate::contract::instantiate;
    use crate::delegations::storage as delegations_storage;
    use crate::gateways::transactions::try_add_gateway;
    use crate::mixnodes::storage as mixnodes_storage;
    use crate::mixnodes::transactions::try_add_mixnode;
    use crate::support::tests;
    use config::defaults::DENOM;
    use cosmwasm_std::coin;
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::testing::MockApi;
    use cosmwasm_std::testing::MockQuerier;
    use cosmwasm_std::Coin;
    use cosmwasm_std::DepsMut;
    use cosmwasm_std::OwnedDeps;
    use cosmwasm_std::{Addr, StdResult, Storage};
    use cosmwasm_std::{Empty, MemoryStorage};
    use cw_storage_plus::PrimaryKey;
    use mixnet_contract_common::{Delegation, Gateway, IdentityKeyRef, InstantiateMsg, MixNode};
    use rand::thread_rng;

    pub fn add_mixnode(sender: &str, stake: Vec<Coin>, deps: DepsMut) -> String {
        let keypair = crypto::asymmetric::identity::KeyPair::new(&mut thread_rng());
        let owner_signature = keypair
            .private_key()
            .sign(sender.as_bytes())
            .to_base58_string();

        let info = mock_info(sender, &stake);
        let key = keypair.public_key().to_base58_string();

        try_add_mixnode(
            deps,
            mock_env(),
            info,
            MixNode {
                identity_key: key.clone(),
                ..tests::fixtures::mix_node_fixture()
            },
            owner_signature,
        )
        .unwrap();
        key
    }

    pub fn add_gateway(sender: &str, stake: Vec<Coin>, deps: DepsMut) -> String {
        let keypair = crypto::asymmetric::identity::KeyPair::new(&mut thread_rng());
        let owner_signature = keypair
            .private_key()
            .sign(sender.as_bytes())
            .to_base58_string();

        let info = mock_info(sender, &stake);
        let key = keypair.public_key().to_base58_string();
        try_add_gateway(
            deps,
            mock_env(),
            info,
            Gateway {
                identity_key: key.clone(),
                ..tests::fixtures::gateway_fixture()
            },
            owner_signature,
        )
        .unwrap();
        key
    }

    pub fn init_contract() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            rewarding_validator_address: config::defaults::DEFAULT_REWARDING_VALIDATOR.to_string(),
        };
        let env = mock_env();
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        deps
    }

    // currently not used outside tests
    pub(crate) fn read_mixnode_pledge_amount(
        storage: &dyn Storage,
        identity: IdentityKeyRef,
    ) -> StdResult<cosmwasm_std::Uint128> {
        let node = mixnodes_storage::mixnodes().load(storage, identity)?;
        Ok(node.pledge_amount.amount)
    }

    pub(crate) fn save_dummy_delegation(
        storage: &mut dyn Storage,
        mix: impl Into<String>,
        owner: impl Into<String>,
    ) {
        let delegation = Delegation {
            owner: Addr::unchecked(owner.into()),
            node_identity: mix.into(),
            amount: coin(12345, DENOM),
            block_height: 12345,
            proxy: None,
        };

        delegations_storage::delegations()
            .save(storage, delegation.storage_key().joined_key(), &delegation)
            .unwrap();
    }

    pub(crate) fn read_delegation(
        storage: &dyn Storage,
        mix: impl Into<String>,
        owner: impl Into<String>,
    ) -> Option<Delegation> {
        delegations_storage::delegations()
            .may_load(storage, (mix.into(), owner.into()).joined_key())
            .unwrap()
    }
}

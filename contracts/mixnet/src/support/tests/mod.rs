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
    use crate::interval;
    use crate::interval::storage as interval_storage;
    use crate::mixnodes::storage as mixnodes_storage;
    use crate::mixnodes::transactions::try_add_mixnode;
    use crate::support::tests;
    use config::defaults::{DEFAULT_NETWORK, DENOM};
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::testing::MockApi;
    use cosmwasm_std::testing::MockQuerier;
    use cosmwasm_std::Coin;
    use cosmwasm_std::DepsMut;
    use cosmwasm_std::OwnedDeps;
    use cosmwasm_std::{coin, Env, Timestamp};
    use cosmwasm_std::{Addr, StdResult, Storage};
    use cosmwasm_std::{Empty, MemoryStorage};
    use mixnet_contract_common::{Delegation, Gateway, IdentityKeyRef, InstantiateMsg, MixNode};
    use rand::thread_rng;

    pub fn add_mixnode(sender: &str, stake: Vec<Coin>, deps: DepsMut<'_>) -> String {
        let keypair = crypto::asymmetric::identity::KeyPair::new(&mut thread_rng());
        let owner_signature = keypair
            .private_key()
            .sign(sender.as_bytes())
            .to_base58_string();

        let legit_sphinx_key = crypto::asymmetric::encryption::KeyPair::new(&mut thread_rng());

        let info = mock_info(sender, &stake);
        let key = keypair.public_key().to_base58_string();

        try_add_mixnode(
            deps,
            mock_env(),
            info,
            MixNode {
                identity_key: key.clone(),
                sphinx_key: legit_sphinx_key.public_key().to_base58_string(),
                ..tests::fixtures::mix_node_fixture()
            },
            owner_signature,
        )
        .unwrap();
        key
    }

    pub fn add_gateway(sender: &str, stake: Vec<Coin>, deps: DepsMut<'_>) -> String {
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
            rewarding_validator_address: DEFAULT_NETWORK.rewarding_validator_address().to_string(),
        };
        let env = mock_env();
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        deps
    }

    // currently not used outside tests
    pub(crate) fn read_mixnode_pledge_amount(
        storage: &dyn Storage,
        identity: IdentityKeyRef<'_>,
    ) -> StdResult<cosmwasm_std::Uint128> {
        let node = mixnodes_storage::mixnodes().load(storage, identity)?;
        Ok(node.pledge_amount.amount)
    }

    pub(crate) fn save_dummy_delegation(
        storage: &mut dyn Storage,
        mix: impl Into<String>,
        owner: impl Into<String>,
        block_height: u64,
    ) {
        let delegation = Delegation {
            owner: Addr::unchecked(owner.into()),
            node_identity: mix.into(),
            amount: coin(12345, DENOM),
            block_height: block_height,
            proxy: None,
        };

        delegations_storage::delegations()
            .save(storage, delegation.storage_key(), &delegation)
            .unwrap();
    }

    pub(crate) fn read_delegation(
        storage: &dyn Storage,
        mix: impl Into<String>,
        owner: impl Into<Vec<u8>>,
        block_height: u64,
    ) -> Option<Delegation> {
        delegations_storage::delegations()
            .may_load(storage, (mix.into(), owner.into(), block_height))
            .unwrap()
    }

    pub(crate) fn update_env_and_progress_interval(env: &mut Env, storage: &mut dyn Storage) {
        // make sure current block time is within the expected next interval
        env.block.time = Timestamp::from_seconds(
            (interval_storage::current_epoch(storage)
                .unwrap()
                .next()
                .start_unix_timestamp()
                + 123) as u64,
        );

        let sender =
            crate::mixnet_contract_settings::storage::rewarding_validator_address(storage).unwrap();

        interval::transactions::try_advance_epoch(env.clone(), storage, sender).unwrap();
    }
}

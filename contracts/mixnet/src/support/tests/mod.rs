// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
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
    use crate::rewards::storage as rewards_storage;
    use crate::support::tests;
    use crate::support::tests::fixtures::TEST_COIN_DENOM;
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::testing::mock_env;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::testing::MockApi;
    use cosmwasm_std::testing::MockQuerier;
    use cosmwasm_std::DepsMut;
    use cosmwasm_std::OwnedDeps;
    use cosmwasm_std::{coin, Env, Timestamp};
    use cosmwasm_std::{Addr, StdResult, Storage};
    use cosmwasm_std::{Coin, Order};
    use cosmwasm_std::{Decimal, Empty, MemoryStorage};
    use mixnet_contract_common::mixnode::UnbondedMixnode;
    use mixnet_contract_common::{
        Delegation, Gateway, InitialRewardingParams, InstantiateMsg, MixNode, NodeId, Percent,
    };
    use rand_chacha::rand_core::{CryptoRng, RngCore, SeedableRng};
    use rand_chacha::ChaCha20Rng;
    use std::time::Duration;

    // use rng with constant seed for all tests so that they would be deterministic
    pub fn test_rng() -> ChaCha20Rng {
        let dummy_seed = [42u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
        rng
    }

    pub fn add_dummy_mixnodes(mut rng: impl RngCore + CryptoRng, mut deps: DepsMut<'_>, n: usize) {
        for i in 0..n {
            add_mixnode(
                &mut rng,
                deps.branch(),
                &format!("owner{}", i),
                tests::fixtures::good_mixnode_pledge(),
            );
        }
    }

    pub fn add_dummy_gateways(mut rng: impl RngCore + CryptoRng, mut deps: DepsMut<'_>, n: usize) {
        for i in 0..n {
            add_gateway(
                &mut rng,
                deps.branch(),
                &format!("owner{}", i),
                tests::fixtures::good_mixnode_pledge(),
            );
        }
    }

    pub fn add_dummy_unbonded_mixnodes(
        mut rng: impl RngCore + CryptoRng,
        mut deps: DepsMut<'_>,
        n: usize,
    ) {
        for i in 0..n {
            add_unbonded_mixnode(&mut rng, deps.branch(), &format!("owner{}", i));
        }
    }

    // same note as with `add_mixnode`
    pub fn add_unbonded_mixnode(
        mut rng: impl RngCore + CryptoRng,
        deps: DepsMut<'_>,
        owner: &str,
    ) -> NodeId {
        let keypair = crypto::asymmetric::identity::KeyPair::new(&mut rng);

        let id = loop {
            let candidate = rng.next_u64();
            if !mixnodes_storage::UNBONDED_MIXNODES.has(deps.storage, candidate) {
                break candidate;
            }
        };

        // we don't care about 'correctness' of the identity key here
        mixnodes_storage::UNBONDED_MIXNODES
            .save(
                deps.storage,
                id,
                &UnbondedMixnode {
                    identity: format!("identity{}", id),
                    owner: Addr::unchecked(owner),
                    unbonding_height: 12345,
                },
            )
            .unwrap();

        id
    }

    // note to whoever wants to refactor this function, you dont want to grab rng here directly
    // via `let rng = test_rng()`
    // because it's extremely likely you might end up calling `add_mixnode()` multiple times
    // in the same test and thus you're going to get mixnodes with the same keys and that's
    // not what you want (presumably)
    pub fn add_mixnode(
        mut rng: impl RngCore + CryptoRng,
        deps: DepsMut<'_>,
        sender: &str,
        stake: Vec<Coin>,
    ) -> NodeId {
        let keypair = crypto::asymmetric::identity::KeyPair::new(&mut rng);
        let owner_signature = keypair
            .private_key()
            .sign(sender.as_bytes())
            .to_base58_string();

        let legit_sphinx_key = crypto::asymmetric::encryption::KeyPair::new(&mut rng);

        let info = mock_info(sender, &stake);
        let key = keypair.public_key().to_base58_string();
        let current_id_counter = mixnodes_storage::MIXNODE_ID_COUNTER
            .may_load(deps.storage)
            .unwrap()
            .unwrap_or_default();

        try_add_mixnode(
            deps,
            mock_env(),
            info,
            MixNode {
                identity_key: key.clone(),
                sphinx_key: legit_sphinx_key.public_key().to_base58_string(),
                ..tests::fixtures::mix_node_fixture()
            },
            tests::fixtures::mix_node_cost_params_fixture(),
            owner_signature,
        )
        .unwrap();

        // newly added mixnode gets assigned the current counter + 1
        current_id_counter + 1
    }

    // same note as with `add_mixnode`
    pub fn add_gateway(
        mut rng: impl RngCore + CryptoRng,
        deps: DepsMut<'_>,
        sender: &str,
        stake: Vec<Coin>,
    ) -> String {
        let keypair = crypto::asymmetric::identity::KeyPair::new(&mut rng);
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

    fn initial_rewarding_params() -> InitialRewardingParams {
        let reward_pool = 250_000_000_000_000u128;
        let staking_supply = 100_000_000_000_000u128;

        InitialRewardingParams {
            initial_reward_pool: Decimal::from_atomics(reward_pool, 0).unwrap(), // 250M * 1M (we're expressing it all in base tokens)
            initial_staking_supply: Decimal::from_atomics(staking_supply, 0).unwrap(), // 100M * 1M
            sybil_resistance: Percent::from_percentage_value(30).unwrap(),
            active_set_work_factor: Decimal::percent(1000), // value '10'
            interval_pool_emission: Percent::from_percentage_value(2).unwrap(),
            rewarded_set_size: 240,
            active_set_size: 100,
        }
    }

    pub fn init_contract() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier<Empty>> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            rewarding_validator_address: "rewarder".into(),
            vesting_contract_address: "vesting-contract".to_string(),
            rewarding_denom: TEST_COIN_DENOM.to_string(),
            epochs_in_interval: 720,
            epoch_duration: Duration::from_secs(60 * 60),
            initial_rewarding_params: initial_rewarding_params(),
        };
        let env = mock_env();
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), env, info, msg).unwrap();
        deps
    }

    // // currently not used outside tests
    // pub(crate) fn read_mixnode_pledge_amount(
    //     storage: &dyn Storage,
    //     identity: IdentityKeyRef<'_>,
    // ) -> StdResult<cosmwasm_std::Uint128> {
    //     let node = mixnodes_storage::mixnodes().load(storage, identity)?;
    //     Ok(node.pledge_amount.amount)
    // }

    pub(crate) fn save_dummy_delegation(
        storage: &mut dyn Storage,
        mix: NodeId,
        owner: impl Into<String>,
    ) {
        let delegation = Delegation {
            owner: Addr::unchecked(owner.into()),
            node_id: mix,
            cumulative_reward_ratio: Default::default(),
            amount: coin(12345, TEST_COIN_DENOM),
            height: 12345,
            proxy: None,
        };

        delegations_storage::delegations()
            .save(storage, delegation.storage_key(), &delegation)
            .unwrap();
    }

    pub(crate) fn read_delegation(
        storage: &dyn Storage,
        mix: NodeId,
        owner: &Addr,
        proxy: &Option<Addr>,
    ) -> Option<Delegation> {
        delegations_storage::delegations()
            .may_load(
                storage,
                Delegation::generate_storage_key(mix, owner, proxy.as_ref()),
            )
            .unwrap()
    }

    pub(crate) fn update_env_and_progress_epoch(deps: DepsMut<'_>, env: &mut Env) {
        // make sure current block time is within the expected next interval
        env.block.time = Timestamp::from_seconds(
            (interval_storage::current_interval(deps.storage)
                .unwrap()
                .current_epoch_end_unix_timestamp()
                + 123) as u64,
        );

        let sender =
            crate::mixnet_contract_settings::storage::rewarding_validator_address(deps.storage)
                .unwrap();
        let active_set_size = rewards_storage::REWARDING_PARAMS
            .load(deps.storage)
            .unwrap()
            .active_set_size;

        // don't bother updating the rewarded set, use what we have right now
        let rewarded_set = interval_storage::REWARDED_SET
            .range(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        let active = rewarded_set
            .iter()
            .filter(|(id, status)| status.is_active())
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();

        let standby = rewarded_set
            .iter()
            .filter(|(id, status)| !status.is_active())
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();

        let new_set = active
            .into_iter()
            .chain(standby.into_iter())
            .collect::<Vec<_>>();

        interval::transactions::try_advance_epoch(
            deps,
            env.clone(),
            mock_info(sender.as_str(), &[]),
            new_set,
            active_set_size,
        )
        .unwrap();
    }
}

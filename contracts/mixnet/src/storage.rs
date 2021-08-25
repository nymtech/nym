// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::queries;
use crate::state::State;
use cosmwasm_std::{Decimal, Order, StdResult, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
use mixnet_contract::{
    Addr, GatewayBond, IdentityKey, IdentityKeyRef, Layer, LayerDistribution, MixNodeBond,
    StateParams,
};

// storage prefixes
// all of them must be unique and presumably not be a prefix of a different one
// keeping them as short as possible is also desirable as they are part of each stored key
// it's not as important for singletons, but is a nice optimisation for buckets

// singletons
const CONFIG_KEY: &[u8] = b"config";
const LAYER_DISTRIBUTION_KEY: &[u8] = b"layers";

// buckets
const PREFIX_MIXNODES: &[u8] = b"mn";
const PREFIX_MIXNODES_OWNERS: &[u8] = b"mo";
const PREFIX_GATEWAYS: &[u8] = b"gt";
const PREFIX_GATEWAYS_OWNERS: &[u8] = b"go";

const PREFIX_MIX_DELEGATION: &[u8] = b"md";
const PREFIX_GATEWAY_DELEGATION: &[u8] = b"gd";
const PREFIX_MIX_REVERSE_DELEGATION: &[u8] = b"dm";
const PREFIX_GATEWAY_REVERSE_DELEGATION: &[u8] = b"dg";

// Contract-level stuff

pub fn config(storage: &mut dyn Storage) -> Singleton<State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read(storage: &dyn Storage) -> ReadonlySingleton<State> {
    singleton_read(storage, CONFIG_KEY)
}

pub(crate) fn read_state_params(storage: &dyn Storage) -> StateParams {
    // note: In any other case, I wouldn't have attempted to unwrap this result, but in here
    // if we fail to load the stored state we would already be in the undefined behaviour land,
    // so we better just blow up immediately.
    config_read(storage).load().unwrap().params
}

pub(crate) fn read_mixnode_epoch_bond_reward_rate(storage: &dyn Storage) -> Decimal {
    // same justification as in `read_state_params` for the unwrap
    config_read(storage)
        .load()
        .unwrap()
        .mixnode_epoch_bond_reward
}

pub(crate) fn read_gateway_epoch_bond_reward_rate(storage: &dyn Storage) -> Decimal {
    // same justification as in `read_state_params` for the unwrap
    config_read(storage)
        .load()
        .unwrap()
        .gateway_epoch_bond_reward
}

pub(crate) fn read_mixnode_epoch_delegation_reward_rate(storage: &dyn Storage) -> Decimal {
    // same justification as in `read_state_params` for the unwrap
    config_read(storage)
        .load()
        .unwrap()
        .mixnode_epoch_delegation_reward
}

pub(crate) fn read_gateway_epoch_delegation_reward_rate(storage: &dyn Storage) -> Decimal {
    // same justification as in `read_state_params` for the unwrap
    config_read(storage)
        .load()
        .unwrap()
        .gateway_epoch_delegation_reward
}

pub fn layer_distribution(storage: &mut dyn Storage) -> Singleton<LayerDistribution> {
    singleton(storage, LAYER_DISTRIBUTION_KEY)
}

pub fn layer_distribution_read(storage: &dyn Storage) -> ReadonlySingleton<LayerDistribution> {
    singleton_read(storage, LAYER_DISTRIBUTION_KEY)
}

pub(crate) fn read_layer_distribution(storage: &dyn Storage) -> LayerDistribution {
    // note: In any other case, I wouldn't have attempted to unwrap this result, but in here
    // if we fail to load the stored state we would already be in the undefined behaviour land,
    // so we better just blow up immediately.
    layer_distribution_read(storage).load().unwrap()
}

pub fn increment_layer_count(storage: &mut dyn Storage, layer: Layer) -> StdResult<()> {
    let mut distribution = layer_distribution(storage).load()?;
    match layer {
        Layer::Gateway => distribution.gateways += 1,
        Layer::One => distribution.layer1 += 1,
        Layer::Two => distribution.layer2 += 1,
        Layer::Three => distribution.layer3 += 1,
    }
    layer_distribution(storage).save(&distribution)
}

pub fn decrement_layer_count(storage: &mut dyn Storage, layer: Layer) -> StdResult<()> {
    let mut distribution = layer_distribution(storage).load()?;
    // It can't possibly go below zero, if it does, it means there's a serious error in the contract logic
    match layer {
        Layer::Gateway => {
            distribution.gateways = distribution
                .gateways
                .checked_sub(1)
                .expect("tried to subtract from unsigned zero!")
        }
        Layer::One => {
            distribution.layer1 = distribution
                .layer1
                .checked_sub(1)
                .expect("tried to subtract from unsigned zero!")
        }
        Layer::Two => {
            distribution.layer2 = distribution
                .layer2
                .checked_sub(1)
                .expect("tried to subtract from unsigned zero!")
        }
        Layer::Three => {
            distribution.layer3 = distribution
                .layer3
                .checked_sub(1)
                .expect("tried to subtract from unsigned zero!")
        }
    };
    layer_distribution(storage).save(&distribution)
}

// Mixnode-related stuff

pub fn mixnodes(storage: &mut dyn Storage) -> Bucket<MixNodeBond> {
    bucket(storage, PREFIX_MIXNODES)
}

pub fn mixnodes_read(storage: &dyn Storage) -> ReadonlyBucket<MixNodeBond> {
    bucket_read(storage, PREFIX_MIXNODES)
}

// owner address -> node identity
pub fn mixnodes_owners(storage: &mut dyn Storage) -> Bucket<IdentityKey> {
    bucket(storage, PREFIX_MIXNODES_OWNERS)
}

pub fn mixnodes_owners_read(storage: &dyn Storage) -> ReadonlyBucket<IdentityKey> {
    bucket_read(storage, PREFIX_MIXNODES_OWNERS)
}

// helpers
pub(crate) fn increase_mix_delegated_stakes(
    storage: &mut dyn Storage,
    mix_identity: IdentityKeyRef,
    scaled_reward_rate: Decimal,
) -> StdResult<Uint128> {
    let chunk_size = queries::DELEGATION_PAGE_MAX_LIMIT as usize;

    let mut total_rewarded = Uint128::zero();
    let mut chunk_start: Option<Vec<_>> = None;
    loop {
        // get `chunk_size` of delegations
        let delegations_chunk = mix_delegations_read(storage, mix_identity)
            .range(chunk_start.as_deref(), None, Order::Ascending)
            .take(chunk_size)
            .collect::<StdResult<Vec<_>>>()?;

        if delegations_chunk.is_empty() {
            break;
        }

        // append 0 byte to the last value to start with whatever is the next succeeding key
        chunk_start = Some(
            delegations_chunk
                .last()
                .unwrap()
                .0
                .iter()
                .cloned()
                .chain(std::iter::once(0u8))
                .collect(),
        );

        // and for each of them increase the stake proportionally to the reward
        for (delegator_address, amount) in delegations_chunk.into_iter() {
            let reward = amount * scaled_reward_rate;
            let new_amount = amount + reward;
            total_rewarded += reward;
            mix_delegations(storage, mix_identity).save(&delegator_address, &new_amount)?;
        }
    }

    Ok(total_rewarded)
}

pub(crate) fn increase_gateway_delegated_stakes(
    storage: &mut dyn Storage,
    gateway_identity: IdentityKeyRef,
    scaled_reward_rate: Decimal,
) -> StdResult<Uint128> {
    let chunk_size = queries::DELEGATION_PAGE_MAX_LIMIT as usize;

    let mut total_rewarded = Uint128::zero();
    let mut chunk_start: Option<Vec<_>> = None;
    loop {
        // get `chunk_size` of delegations
        let delegations_chunk = gateway_delegations_read(storage, gateway_identity)
            .range(chunk_start.as_deref(), None, Order::Ascending)
            .take(chunk_size)
            .collect::<StdResult<Vec<_>>>()?;

        if delegations_chunk.is_empty() {
            break;
        }

        // append 0 byte to the last value to start with whatever is the next suceeding key
        chunk_start = Some(
            delegations_chunk
                .last()
                .unwrap()
                .0
                .iter()
                .cloned()
                .chain(std::iter::once(0u8))
                .collect(),
        );

        // and for each of them increase the stake proportionally to the reward
        for (delegator_address, amount) in delegations_chunk.into_iter() {
            let reward = amount * scaled_reward_rate;
            let new_amount = amount + reward;
            total_rewarded += reward;
            gateway_delegations(storage, gateway_identity).save(&delegator_address, &new_amount)?;
        }
    }

    Ok(total_rewarded)
}

// currently not used outside tests
#[cfg(test)]
pub(crate) fn read_mixnode_bond(
    storage: &dyn Storage,
    identity: &[u8],
) -> StdResult<cosmwasm_std::Uint128> {
    let bucket = mixnodes_read(storage);
    let node = bucket.load(identity)?;
    Ok(node.bond_amount.amount)
}

// currently not used outside tests
#[cfg(test)]
pub(crate) fn read_mixnode_delegation(
    storage: &dyn Storage,
    identity: &[u8],
) -> StdResult<cosmwasm_std::Uint128> {
    let bucket = mixnodes_read(storage);
    let node = bucket.load(identity)?;
    Ok(node.total_delegation.amount)
}

// Gateway-related stuff

pub fn gateways(storage: &mut dyn Storage) -> Bucket<GatewayBond> {
    bucket(storage, PREFIX_GATEWAYS)
}

pub fn gateways_read(storage: &dyn Storage) -> ReadonlyBucket<GatewayBond> {
    bucket_read(storage, PREFIX_GATEWAYS)
}

// owner address -> node identity
pub fn gateways_owners(storage: &mut dyn Storage) -> Bucket<IdentityKey> {
    bucket(storage, PREFIX_GATEWAYS_OWNERS)
}

pub fn gateways_owners_read(storage: &dyn Storage) -> ReadonlyBucket<IdentityKey> {
    bucket_read(storage, PREFIX_GATEWAYS_OWNERS)
}

// delegation related
pub fn mix_delegations<'a>(
    storage: &'a mut dyn Storage,
    mix_identity: IdentityKeyRef,
) -> Bucket<'a, Uint128> {
    Bucket::multilevel(storage, &[PREFIX_MIX_DELEGATION, mix_identity.as_bytes()])
}

pub fn mix_delegations_read<'a>(
    storage: &'a dyn Storage,
    mix_identity: IdentityKeyRef,
) -> ReadonlyBucket<'a, Uint128> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_MIX_DELEGATION, mix_identity.as_bytes()])
}

pub fn mix_reverse_delegations<'a>(storage: &'a mut dyn Storage, owner: &Addr) -> Bucket<'a, ()> {
    Bucket::multilevel(storage, &[PREFIX_MIX_REVERSE_DELEGATION, owner.as_bytes()])
}

pub fn mix_reverse_delegations_read<'a>(
    storage: &'a dyn Storage,
    owner: &Addr,
) -> ReadonlyBucket<'a, ()> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_MIX_REVERSE_DELEGATION, owner.as_bytes()])
}

pub fn gateway_delegations<'a>(
    storage: &'a mut dyn Storage,
    gateway_identity: IdentityKeyRef,
) -> Bucket<'a, Uint128> {
    Bucket::multilevel(
        storage,
        &[PREFIX_GATEWAY_DELEGATION, gateway_identity.as_bytes()],
    )
}

pub fn gateway_delegations_read<'a>(
    storage: &'a dyn Storage,
    gateway_identity: IdentityKeyRef,
) -> ReadonlyBucket<'a, Uint128> {
    ReadonlyBucket::multilevel(
        storage,
        &[PREFIX_GATEWAY_DELEGATION, gateway_identity.as_bytes()],
    )
}

pub fn gateway_reverse_delegations<'a>(
    storage: &'a mut dyn Storage,
    owner: &Addr,
) -> Bucket<'a, ()> {
    Bucket::multilevel(
        storage,
        &[PREFIX_GATEWAY_REVERSE_DELEGATION, owner.as_bytes()],
    )
}

pub fn gateway_reverse_delegations_read<'a>(
    storage: &'a dyn Storage,
    owner: &Addr,
) -> ReadonlyBucket<'a, ()> {
    ReadonlyBucket::multilevel(
        storage,
        &[PREFIX_GATEWAY_REVERSE_DELEGATION, owner.as_bytes()],
    )
}

// currently not used outside tests
#[cfg(test)]
pub(crate) fn read_gateway_bond(
    storage: &dyn Storage,
    identity: &[u8],
) -> StdResult<cosmwasm_std::Uint128> {
    let bucket = gateways_read(storage);
    let node = bucket.load(identity)?;
    Ok(node.bond_amount.amount)
}

// currently not used outside tests
#[cfg(test)]
pub(crate) fn read_gateway_delegation(
    storage: &dyn Storage,
    identity: &[u8],
) -> StdResult<cosmwasm_std::Uint128> {
    let bucket = gateways_read(storage);
    let node = bucket.load(identity)?;
    Ok(node.total_delegation.amount)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::helpers::{
        gateway_bond_fixture, gateway_fixture, mix_node_fixture, mixnode_bond_fixture,
    };
    use config::defaults::DENOM;
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::{coin, Addr, Uint128};
    use mixnet_contract::{Gateway, MixNode};

    #[test]
    fn mixnode_single_read_retrieval() {
        let mut storage = MockStorage::new();
        let bond1 = mixnode_bond_fixture();
        let bond2 = mixnode_bond_fixture();
        mixnodes(&mut storage).save(b"bond1", &bond1).unwrap();
        mixnodes(&mut storage).save(b"bond2", &bond2).unwrap();

        let res1 = mixnodes_read(&storage).load(b"bond1").unwrap();
        let res2 = mixnodes_read(&storage).load(b"bond2").unwrap();
        assert_eq!(bond1, res1);
        assert_eq!(bond2, res2);
    }

    #[test]
    fn gateway_single_read_retrieval() {
        let mut storage = MockStorage::new();
        let bond1 = gateway_bond_fixture();
        let bond2 = gateway_bond_fixture();
        gateways(&mut storage).save(b"bond1", &bond1).unwrap();
        gateways(&mut storage).save(b"bond2", &bond2).unwrap();

        let res1 = gateways_read(&storage).load(b"bond1").unwrap();
        let res2 = gateways_read(&storage).load(b"bond2").unwrap();
        assert_eq!(bond1, res1);
        assert_eq!(bond2, res2);
    }

    #[test]
    fn reading_mixnode_bond() {
        let mut storage = MockStorage::new();
        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        // produces an error if target mixnode doesn't exist
        let res = read_mixnode_bond(&storage, node_owner.as_bytes());
        assert!(res.is_err());

        // returns appropriate value otherwise
        let bond_value = 1000;

        let mixnode_bond = MixNodeBond {
            bond_amount: coin(bond_value, DENOM),
            total_delegation: coin(0, DENOM),
            owner: node_owner.clone(),
            layer: Layer::One,
            mix_node: MixNode {
                identity_key: node_identity.clone(),
                ..mix_node_fixture()
            },
        };

        mixnodes(&mut storage)
            .save(node_identity.as_bytes(), &mixnode_bond)
            .unwrap();

        assert_eq!(
            Uint128(bond_value),
            read_mixnode_bond(&storage, node_identity.as_bytes()).unwrap()
        );
    }

    #[test]
    fn reading_gateway_bond() {
        let mut storage = MockStorage::new();
        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        // produces an error if target gateway doesn't exist
        let res = read_gateway_bond(&storage, node_owner.as_bytes());
        assert!(res.is_err());

        // returns appropriate value otherwise
        let bond_value = 1000;

        let gateway_bond = GatewayBond {
            bond_amount: coin(bond_value, DENOM),
            total_delegation: coin(0, DENOM),
            owner: node_owner.clone(),
            gateway: Gateway {
                identity_key: node_identity.clone(),
                ..gateway_fixture()
            },
        };

        gateways(&mut storage)
            .save(node_identity.as_bytes(), &gateway_bond)
            .unwrap();

        assert_eq!(
            Uint128(bond_value),
            read_gateway_bond(&storage, node_identity.as_bytes()).unwrap()
        );
    }

    #[cfg(test)]
    mod increasing_mix_delegated_stakes {
        use super::*;
        use crate::queries::query_mixnode_delegations_paged;
        use cosmwasm_std::testing::mock_dependencies;

        #[test]
        fn when_there_are_no_delegations() {
            let mut deps = mock_dependencies(&[]);
            let node_identity: IdentityKey = "nodeidentity".into();

            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            let total_increase =
                increase_mix_delegated_stakes(&mut deps.storage, node_identity.as_ref(), reward)
                    .unwrap();

            // there was no increase
            assert!(total_increase.is_zero());

            // there are no 'new' delegations magically added
            assert!(
                query_mixnode_delegations_paged(deps.as_ref(), node_identity, None, None)
                    .unwrap()
                    .delegations
                    .is_empty()
            )
        }

        #[test]
        fn when_there_is_a_single_delegation() {
            let mut deps = mock_dependencies(&[]);
            let node_identity: IdentityKey = "nodeidentity".into();

            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            let delegator_address = Addr::unchecked("bob");
            mix_delegations(&mut deps.storage, &node_identity)
                .save(delegator_address.as_bytes(), &Uint128(1000))
                .unwrap();

            let total_increase =
                increase_mix_delegated_stakes(&mut deps.storage, node_identity.as_ref(), reward)
                    .unwrap();

            assert_eq!(Uint128(1), total_increase);

            assert_eq!(
                Uint128(1001),
                mix_delegations_read(&mut deps.storage, &node_identity)
                    .load(delegator_address.as_bytes())
                    .unwrap()
            )
        }

        #[test]
        fn when_there_are_multiple_delegations() {
            let mut deps = mock_dependencies(&[]);
            let node_identity: IdentityKey = "nodeidentity".into();

            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            for i in 0..100 {
                let delegator_address = Addr::unchecked(format!("address{}", i));
                mix_delegations(&mut deps.storage, &node_identity)
                    .save(delegator_address.as_bytes(), &Uint128(1000))
                    .unwrap();
            }

            let total_increase =
                increase_mix_delegated_stakes(&mut deps.storage, node_identity.as_ref(), reward)
                    .unwrap();

            assert_eq!(Uint128(100), total_increase);

            for i in 0..100 {
                let delegator_address = Addr::unchecked(format!("address{}", i));
                assert_eq!(
                    Uint128(1001),
                    mix_delegations_read(&mut deps.storage, &node_identity)
                        .load(delegator_address.as_bytes())
                        .unwrap()
                )
            }
        }

        #[test]
        fn when_there_are_more_delegations_than_page_size() {
            let mut deps = mock_dependencies(&[]);
            let node_identity: IdentityKey = "nodeidentity".into();

            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            for i in 0..queries::DELEGATION_PAGE_MAX_LIMIT * 10 {
                let delegator_address = Addr::unchecked(format!("address{}", i));
                mix_delegations(&mut deps.storage, &node_identity)
                    .save(delegator_address.as_bytes(), &Uint128(1000))
                    .unwrap();
            }

            let total_increase =
                increase_mix_delegated_stakes(&mut deps.storage, node_identity.as_ref(), reward)
                    .unwrap();

            assert_eq!(
                Uint128(queries::DELEGATION_PAGE_MAX_LIMIT as u128 * 10),
                total_increase
            );

            for i in 0..queries::DELEGATION_PAGE_MAX_LIMIT * 10 {
                let delegator_address = Addr::unchecked(format!("address{}", i));
                assert_eq!(
                    Uint128(1001),
                    mix_delegations_read(&mut deps.storage, &node_identity)
                        .load(delegator_address.as_bytes())
                        .unwrap()
                )
            }
        }
    }

    #[cfg(test)]
    mod mix_reverse_delegations {
        use super::*;
        use crate::support::tests::helpers;

        #[test]
        fn mix_reverse_delegation_exists() {
            let mut deps = helpers::init_contract();
            let node_identity: IdentityKey = "foo".into();
            let delegation_owner = Addr::unchecked("bar");

            mix_reverse_delegations(&mut deps.storage, &delegation_owner)
                .save(node_identity.as_bytes(), &())
                .unwrap();

            assert!(
                mix_reverse_delegations_read(deps.as_ref().storage, &delegation_owner)
                    .may_load(node_identity.as_bytes())
                    .unwrap()
                    .is_some(),
            );
        }

        #[test]
        fn mix_reverse_delegation_returns_none_if_delegation_doesnt_exist() {
            let mut deps = helpers::init_contract();

            let node_identity1: IdentityKey = "foo1".into();
            let node_identity2: IdentityKey = "foo2".into();
            let delegation_owner1 = Addr::unchecked("bar");
            let delegation_owner2 = Addr::unchecked("bar2");

            assert!(
                mix_reverse_delegations_read(deps.as_ref().storage, &delegation_owner1)
                    .may_load(node_identity1.as_bytes())
                    .unwrap()
                    .is_none()
            );

            // add delegation for a different node
            mix_reverse_delegations(&mut deps.storage, &delegation_owner1)
                .save(node_identity2.as_bytes(), &())
                .unwrap();

            assert!(
                mix_reverse_delegations_read(deps.as_ref().storage, &delegation_owner1)
                    .may_load(node_identity1.as_bytes())
                    .unwrap()
                    .is_none()
            );

            // add delegation from a different owner
            mix_reverse_delegations(&mut deps.storage, &delegation_owner2)
                .save(node_identity1.as_bytes(), &())
                .unwrap();

            assert!(
                mix_reverse_delegations_read(deps.as_ref().storage, &delegation_owner1)
                    .may_load(node_identity1.as_bytes())
                    .unwrap()
                    .is_none()
            );
        }
    }

    #[cfg(test)]
    mod increasing_gateway_delegated_stakes {
        use super::*;
        use crate::queries::query_gateway_delegations_paged;
        use cosmwasm_std::testing::mock_dependencies;

        #[test]
        fn when_there_are_no_delegations() {
            let mut deps = mock_dependencies(&[]);
            let node_identity: IdentityKey = "nodeidentity".into();

            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            let total_increase = increase_gateway_delegated_stakes(
                &mut deps.storage,
                node_identity.as_ref(),
                reward,
            )
            .unwrap();

            // there was no increase
            assert!(total_increase.is_zero());

            // there are no 'new' delegations magically added
            assert!(
                query_gateway_delegations_paged(deps.as_ref(), node_identity, None, None)
                    .unwrap()
                    .delegations
                    .is_empty()
            )
        }

        #[test]
        fn when_there_is_a_single_delegation() {
            let mut deps = mock_dependencies(&[]);
            let node_identity: IdentityKey = "nodeidentity".into();

            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            let delegator_address = Addr::unchecked("bob");
            gateway_delegations(&mut deps.storage, &node_identity)
                .save(delegator_address.as_bytes(), &Uint128(1000))
                .unwrap();

            let total_increase = increase_gateway_delegated_stakes(
                &mut deps.storage,
                node_identity.as_ref(),
                reward,
            )
            .unwrap();

            assert_eq!(Uint128(1), total_increase);

            assert_eq!(
                Uint128(1001),
                gateway_delegations_read(&mut deps.storage, &node_identity)
                    .load(delegator_address.as_bytes())
                    .unwrap()
            )
        }

        #[test]
        fn when_there_are_multiple_delegations() {
            let mut deps = mock_dependencies(&[]);
            let node_identity: IdentityKey = "nodeidentity".into();

            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            for i in 0..100 {
                let delegator_address = Addr::unchecked(format!("address{}", i));
                gateway_delegations(&mut deps.storage, &node_identity)
                    .save(delegator_address.as_bytes(), &Uint128(1000))
                    .unwrap();
            }

            let total_increase = increase_gateway_delegated_stakes(
                &mut deps.storage,
                node_identity.as_ref(),
                reward,
            )
            .unwrap();

            assert_eq!(Uint128(100), total_increase);

            for i in 0..100 {
                let delegator_address = Addr::unchecked(format!("address{}", i));
                assert_eq!(
                    Uint128(1001),
                    gateway_delegations_read(&mut deps.storage, &node_identity)
                        .load(delegator_address.as_bytes())
                        .unwrap()
                )
            }
        }

        #[test]
        fn when_there_are_more_delegations_than_page_size() {
            let mut deps = mock_dependencies(&[]);
            let node_identity: IdentityKey = "nodeidentity".into();

            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            for i in 0..queries::DELEGATION_PAGE_MAX_LIMIT * 10 {
                let delegator_address = Addr::unchecked(format!("address{}", i));
                gateway_delegations(&mut deps.storage, &node_identity)
                    .save(delegator_address.as_bytes(), &Uint128(1000))
                    .unwrap();
            }

            let total_increase = increase_gateway_delegated_stakes(
                &mut deps.storage,
                node_identity.as_ref(),
                reward,
            )
            .unwrap();

            assert_eq!(
                Uint128(queries::DELEGATION_PAGE_MAX_LIMIT as u128 * 10),
                total_increase
            );

            for i in 0..queries::DELEGATION_PAGE_MAX_LIMIT * 10 {
                let delegator_address = Addr::unchecked(format!("address{}", i));
                assert_eq!(
                    Uint128(1001),
                    gateway_delegations_read(&mut deps.storage, &node_identity)
                        .load(delegator_address.as_bytes())
                        .unwrap()
                )
            }
        }
    }

    #[cfg(test)]
    mod gateway_reverse_delegations {
        use super::*;
        use crate::support::tests::helpers;

        #[test]
        fn gateway_reverse_delegation_exists() {
            let mut deps = helpers::init_contract();
            let node_identity: IdentityKey = "foo".into();
            let delegation_owner = Addr::unchecked("bar");

            mix_reverse_delegations(&mut deps.storage, &delegation_owner)
                .save(node_identity.as_bytes(), &())
                .unwrap();

            assert!(
                mix_reverse_delegations_read(deps.as_ref().storage, &delegation_owner)
                    .may_load(node_identity.as_bytes())
                    .unwrap()
                    .is_some(),
            );
        }

        #[test]
        fn gateway_reverse_delegation_returns_none_if_delegation_doesnt_exist() {
            let mut deps = helpers::init_contract();

            let node_identity1: IdentityKey = "foo1".into();
            let node_identity2: IdentityKey = "foo2".into();
            let delegation_owner1 = Addr::unchecked("bar");
            let delegation_owner2 = Addr::unchecked("bar2");

            assert!(
                gateway_reverse_delegations_read(deps.as_ref().storage, &delegation_owner1)
                    .may_load(node_identity1.as_bytes())
                    .unwrap()
                    .is_none()
            );

            // add delegation for a different node
            gateway_reverse_delegations(&mut deps.storage, &delegation_owner1)
                .save(node_identity2.as_bytes(), &())
                .unwrap();

            assert!(
                gateway_reverse_delegations_read(deps.as_ref().storage, &delegation_owner1)
                    .may_load(node_identity1.as_bytes())
                    .unwrap()
                    .is_none()
            );

            // add delegation from a different owner
            gateway_reverse_delegations(&mut deps.storage, &delegation_owner2)
                .save(node_identity1.as_bytes(), &())
                .unwrap();

            assert!(
                gateway_reverse_delegations_read(deps.as_ref().storage, &delegation_owner1)
                    .may_load(node_identity1.as_bytes())
                    .unwrap()
                    .is_none()
            );
        }
    }
}

// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use cosmwasm_std::{Order, StdResult, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
use mixnet_contract::IdentityKey;
use std::collections::HashMap;

use crate::vesting::{DelegationData, PeriodicVestingAccount};
// storage prefixes
// all of them must be unique and presumably not be a prefix of a different one
// keeping them as short as possible is also desirable as they are part of each stored key
// it's not as important for singletons, but is a nice optimisation for buckets

// singletons

// buckets
const PREFIX_ACCOUNTS: &[u8] = b"ac";
const PREFIX_ACCOUNT_DELEGATIONS: &[u8] = b"ad";
// Contract-level stuff

pub fn accounts_mut(storage: &mut dyn Storage) -> Bucket<PeriodicVestingAccount> {
    bucket(storage, PREFIX_ACCOUNTS)
}

pub fn accounts(storage: &dyn Storage) -> ReadonlyBucket<PeriodicVestingAccount> {
    bucket_read(storage, PREFIX_ACCOUNTS)
}

pub fn account_delegations_mut(
    storage: &mut dyn Storage,
) -> Bucket<HashMap<IdentityKey, Vec<DelegationData>>> {
    bucket(storage, PREFIX_ACCOUNT_DELEGATIONS)
}

pub fn account_delegations(
    storage: &dyn Storage,
) -> ReadonlyBucket<HashMap<IdentityKey, Vec<DelegationData>>> {
    bucket_read(storage, PREFIX_ACCOUNT_DELEGATIONS)
}

pub fn get_account(storage: &dyn Storage, address: &str) -> Option<PeriodicVestingAccount> {
    // Due to using may_load this should be safe to unwrap
    accounts(storage).may_load(address.as_bytes()).unwrap()
}

pub fn get_account_delegations(
    storage: &dyn Storage,
    address: &str,
) -> Option<HashMap<IdentityKey, Vec<DelegationData>>> {
    // Due to using may_load this should be safe to unwrap
    account_delegations(storage)
        .may_load(address.as_bytes())
        .unwrap()
}

pub fn set_account_delegations(
    storage: &mut dyn Storage,
    address: &str,
    delegations: HashMap<IdentityKey, Vec<DelegationData>>,
) -> StdResult<()> {
    account_delegations_mut(storage).save(address.as_bytes(), &delegations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::identity_and_owner_to_bytes;
    use crate::support::tests::helpers::{
        gateway_bond_fixture, gateway_fixture, mix_node_fixture, mixnode_bond_fixture,
        raw_delegation_fixture,
    };
    use config::defaults::DENOM;
    use cosmwasm_std::testing::{mock_dependencies, MockStorage};
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
            block_height: 12_345,
            mix_node: MixNode {
                identity_key: node_identity.clone(),
                ..mix_node_fixture()
            },
            profit_margin_percent: Some(10),
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
    fn all_mixnode_delegations_read_retrieval() {
        let mut deps = mock_dependencies(&[]);
        let node_identity1: IdentityKey = "foo1".into();
        let delegation_owner1 = Addr::unchecked("bar1");
        let node_identity2: IdentityKey = "foo2".into();
        let delegation_owner2 = Addr::unchecked("bar2");
        let raw_delegation1 = RawDelegationData::new(1u128.into(), 1000);
        let raw_delegation2 = RawDelegationData::new(2u128.into(), 2000);

        mix_delegations(&mut deps.storage, &node_identity1)
            .save(delegation_owner1.as_bytes(), &raw_delegation1)
            .unwrap();
        mix_delegations(&mut deps.storage, &node_identity2)
            .save(delegation_owner2.as_bytes(), &raw_delegation2)
            .unwrap();

        let res1 = all_mix_delegations_read::<RawDelegationData>(&deps.storage)
            .load(&*identity_and_owner_to_bytes(
                &node_identity1,
                &delegation_owner1,
            ))
            .unwrap();
        let res2 = all_mix_delegations_read::<RawDelegationData>(&deps.storage)
            .load(&*identity_and_owner_to_bytes(
                &node_identity2,
                &delegation_owner2,
            ))
            .unwrap();
        assert_eq!(raw_delegation1, res1);
        assert_eq!(raw_delegation2, res2);
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

            let total_increase = increase_mix_delegated_stakes(
                &mut deps.storage,
                node_identity.as_ref(),
                reward,
                42,
            )
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
            let delegation_blockstamp = 42;

            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            let delegator_address = Addr::unchecked("bob");
            mix_delegations(&mut deps.storage, &node_identity)
                .save(
                    delegator_address.as_bytes(),
                    &RawDelegationData::new(1000u128.into(), delegation_blockstamp),
                )
                .unwrap();

            let total_increase = increase_mix_delegated_stakes(
                &mut deps.storage,
                node_identity.as_ref(),
                reward,
                delegation_blockstamp + 2 * MINIMUM_BLOCK_AGE_FOR_REWARDING,
            )
            .unwrap();

            assert_eq!(Uint128(1), total_increase);

            // amount is incremented, block height remains the same
            assert_eq!(
                RawDelegationData::new(1001u128.into(), 42),
                mix_delegations_read(&mut deps.storage, &node_identity)
                    .load(delegator_address.as_bytes())
                    .unwrap()
            )
        }

        #[test]
        fn when_there_is_a_single_delegation_depending_on_blockstamp() {
            let mut deps = mock_dependencies(&[]);
            let node_identity: IdentityKey = "nodeidentity".into();
            let delegation_blockstamp = 42;

            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            let delegator_address = Addr::unchecked("bob");
            mix_delegations(&mut deps.storage, &node_identity)
                .save(
                    delegator_address.as_bytes(),
                    &RawDelegationData::new(1000u128.into(), delegation_blockstamp),
                )
                .unwrap();

            let total_increase = increase_mix_delegated_stakes(
                &mut deps.storage,
                node_identity.as_ref(),
                reward,
                delegation_blockstamp + MINIMUM_BLOCK_AGE_FOR_REWARDING - 1,
            )
            .unwrap();

            // there was no increase
            assert!(total_increase.is_zero());

            // amount is not incremented
            assert_eq!(
                RawDelegationData::new(1000u128.into(), delegation_blockstamp),
                mix_delegations_read(&mut deps.storage, &node_identity)
                    .load(delegator_address.as_bytes())
                    .unwrap()
            );

            let total_increase = increase_mix_delegated_stakes(
                &mut deps.storage,
                node_identity.as_ref(),
                reward,
                delegation_blockstamp + MINIMUM_BLOCK_AGE_FOR_REWARDING,
            )
            .unwrap();

            // there is an increase now, that the lock period has passed
            assert_eq!(Uint128(1), total_increase);

            // amount is incremented
            assert_eq!(
                RawDelegationData::new(1001u128.into(), delegation_blockstamp),
                mix_delegations_read(&mut deps.storage, &node_identity)
                    .load(delegator_address.as_bytes())
                    .unwrap()
            )
        }

        #[test]
        fn when_there_are_multiple_delegations() {
            let mut deps = mock_dependencies(&[]);
            let node_identity: IdentityKey = "nodeidentity".into();
            let delegation_blockstamp = 42;

            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            for i in 0..100 {
                let delegator_address = Addr::unchecked(format!("address{}", i));
                mix_delegations(&mut deps.storage, &node_identity)
                    .save(
                        delegator_address.as_bytes(),
                        &RawDelegationData::new(1000u128.into(), delegation_blockstamp),
                    )
                    .unwrap();
            }

            let total_increase = increase_mix_delegated_stakes(
                &mut deps.storage,
                node_identity.as_ref(),
                reward,
                delegation_blockstamp + 2 * MINIMUM_BLOCK_AGE_FOR_REWARDING,
            )
            .unwrap();

            assert_eq!(Uint128(100), total_increase);

            for i in 0..100 {
                let delegator_address = Addr::unchecked(format!("address{}", i));
                assert_eq!(
                    raw_delegation_fixture(1001),
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
            let delegation_blockstamp = 42;

            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            for i in 0..queries::DELEGATION_PAGE_MAX_LIMIT * 10 {
                let delegator_address = Addr::unchecked(format!("address{}", i));
                mix_delegations(&mut deps.storage, &node_identity)
                    .save(
                        delegator_address.as_bytes(),
                        &RawDelegationData::new(1000u128.into(), delegation_blockstamp),
                    )
                    .unwrap();
            }

            let total_increase = increase_mix_delegated_stakes(
                &mut deps.storage,
                node_identity.as_ref(),
                reward,
                delegation_blockstamp + 2 * MINIMUM_BLOCK_AGE_FOR_REWARDING,
            )
            .unwrap();

            assert_eq!(
                Uint128(queries::DELEGATION_PAGE_MAX_LIMIT as u128 * 10),
                total_increase
            );

            for i in 0..queries::DELEGATION_PAGE_MAX_LIMIT * 10 {
                let delegator_address = Addr::unchecked(format!("address{}", i));
                assert_eq!(
                    raw_delegation_fixture(1001),
                    mix_delegations_read(&mut deps.storage, &node_identity)
                        .load(delegator_address.as_bytes())
                        .unwrap()
                )
            }
        }
    }

    #[cfg(test)]
    mod reverse_mix_delegations {
        use super::*;
        use crate::support::tests::helpers;

        #[test]
        fn reverse_mix_delegation_exists() {
            let mut deps = helpers::init_contract();
            let node_identity: IdentityKey = "foo".into();
            let delegation_owner = Addr::unchecked("bar");

            reverse_mix_delegations(&mut deps.storage, &delegation_owner)
                .save(node_identity.as_bytes(), &())
                .unwrap();

            assert!(
                reverse_mix_delegations_read(deps.as_ref().storage, &delegation_owner)
                    .may_load(node_identity.as_bytes())
                    .unwrap()
                    .is_some(),
            );
        }

        #[test]
        fn reverse_mix_delegation_returns_none_if_delegation_doesnt_exist() {
            let mut deps = helpers::init_contract();

            let node_identity1: IdentityKey = "foo1".into();
            let node_identity2: IdentityKey = "foo2".into();
            let delegation_owner1 = Addr::unchecked("bar");
            let delegation_owner2 = Addr::unchecked("bar2");

            assert!(
                reverse_mix_delegations_read(deps.as_ref().storage, &delegation_owner1)
                    .may_load(node_identity1.as_bytes())
                    .unwrap()
                    .is_none()
            );

            // add delegation for a different node
            reverse_mix_delegations(&mut deps.storage, &delegation_owner1)
                .save(node_identity2.as_bytes(), &())
                .unwrap();

            assert!(
                reverse_mix_delegations_read(deps.as_ref().storage, &delegation_owner1)
                    .may_load(node_identity1.as_bytes())
                    .unwrap()
                    .is_none()
            );

            // add delegation from a different owner
            reverse_mix_delegations(&mut deps.storage, &delegation_owner2)
                .save(node_identity1.as_bytes(), &())
                .unwrap();

            assert!(
                reverse_mix_delegations_read(deps.as_ref().storage, &delegation_owner1)
                    .may_load(node_identity1.as_bytes())
                    .unwrap()
                    .is_none()
            );
        }
    }
}

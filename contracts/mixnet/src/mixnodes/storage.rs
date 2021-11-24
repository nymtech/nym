// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::contract::INITIAL_REWARD_POOL;
use crate::error::ContractError;
use crate::rewards::storage as rewards_storage;
use config::defaults::{DENOM, TOTAL_SUPPLY};
use cosmwasm_std::{Decimal, Order, StdResult, Storage, Uint128};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
use mixnet_contract::mixnode::NodeRewardParams;
use mixnet_contract::{
    Addr, Coin, GatewayBond, IdentityKey, IdentityKeyRef, Layer, LayerDistribution, MixNode,
    MixNodeBond, RawDelegationData, RewardingStatus,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

// storage prefixes
const PREFIX_MIXNODES: &[u8] = b"mn";
const PREFIX_MIXNODES_OWNERS: &[u8] = b"mo";
const PREFIX_MIX_DELEGATION: &[u8] = b"md";
const PREFIX_REVERSE_MIX_DELEGATION: &[u8] = b"dm";
pub const PREFIX_REWARDED_MIXNODES: &[u8] = b"rm";

// paged retrieval limits for all queries and transactions
// currently the maximum limit before running into memory issue is somewhere between 1150 and 1200
pub(crate) const DELEGATION_PAGE_MAX_LIMIT: u32 = 500;
pub(crate) const DELEGATION_PAGE_DEFAULT_LIMIT: u32 = 250;
pub(crate) const BOND_PAGE_MAX_LIMIT: u32 = 75;
pub(crate) const BOND_PAGE_DEFAULT_LIMIT: u32 = 50;

const PREFIX_TOTAL_DELEGATION: &[u8] = b"td";

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub(crate) struct StoredMixnodeBond {
    pub bond_amount: Coin,
    pub owner: Addr,
    pub layer: Layer,
    pub block_height: u64,
    pub mix_node: MixNode,
    pub profit_margin_percent: Option<u8>,
}

impl StoredMixnodeBond {
    pub(crate) fn new(
        bond_amount: Coin,
        owner: Addr,
        layer: Layer,
        block_height: u64,
        mix_node: MixNode,
        profit_margin_percent: Option<u8>,
    ) -> Self {
        StoredMixnodeBond {
            bond_amount,
            owner,
            layer,
            block_height,
            mix_node,
            profit_margin_percent,
        }
    }

    pub(crate) fn attach_delegation(self, total_delegation: Uint128) -> MixNodeBond {
        MixNodeBond {
            total_delegation: Coin {
                denom: self.bond_amount.denom.clone(),
                amount: total_delegation,
            },
            bond_amount: self.bond_amount,
            owner: self.owner,
            layer: self.layer,
            block_height: self.block_height,
            mix_node: self.mix_node,
            profit_margin_percent: self.profit_margin_percent,
        }
    }

    pub(crate) fn identity(&self) -> &String {
        &self.mix_node.identity_key
    }

    pub(crate) fn bond_amount(&self) -> Coin {
        self.bond_amount.clone()
    }
}

impl Display for StoredMixnodeBond {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "amount: {}, owner: {}, identity: {}",
            self.bond_amount, self.owner, self.mix_node.identity_key
        )
    }
}

// Mixnode-related stuff

pub(crate) fn mixnodes(storage: &mut dyn Storage) -> Bucket<StoredMixnodeBond> {
    bucket(storage, PREFIX_MIXNODES)
}

pub(crate) fn mixnodes_read(storage: &dyn Storage) -> ReadonlyBucket<StoredMixnodeBond> {
    bucket_read(storage, PREFIX_MIXNODES)
}

// owner address -> node identity
pub fn mixnodes_owners(storage: &mut dyn Storage) -> Bucket<IdentityKey> {
    bucket(storage, PREFIX_MIXNODES_OWNERS)
}

pub fn mixnodes_owners_read(storage: &dyn Storage) -> ReadonlyBucket<IdentityKey> {
    bucket_read(storage, PREFIX_MIXNODES_OWNERS)
}

pub fn total_delegation(storage: &mut dyn Storage) -> Bucket<Uint128> {
    bucket(storage, PREFIX_TOTAL_DELEGATION)
}

pub fn total_delegation_read(storage: &dyn Storage) -> ReadonlyBucket<Uint128> {
    bucket_read(storage, PREFIX_TOTAL_DELEGATION)
}

// we want to treat this bucket as a set so we don't really care about what type of data is being stored.
// I went with u8 as after serialization it takes only a single byte of space, while if a `()` was used,
// it would have taken 4 bytes (representation of 'null')
pub(crate) fn rewarded_mixnodes(
    storage: &mut dyn Storage,
    rewarding_interval_nonce: u32,
) -> Bucket<RewardingStatus> {
    Bucket::multilevel(
        storage,
        &[
            rewarding_interval_nonce.to_be_bytes().as_ref(),
            PREFIX_REWARDED_MIXNODES,
        ],
    )
}

pub(crate) fn rewarded_mixnodes_read(
    storage: &dyn Storage,
    rewarding_interval_nonce: u32,
) -> ReadonlyBucket<RewardingStatus> {
    ReadonlyBucket::multilevel(
        storage,
        &[
            rewarding_interval_nonce.to_be_bytes().as_ref(),
            PREFIX_REWARDED_MIXNODES,
        ],
    )
}

pub(crate) fn read_mixnode_bond(
    storage: &dyn Storage,
    mix_identity: IdentityKeyRef,
) -> StdResult<Option<MixNodeBond>> {
    let stored_bond = mixnodes_read(storage).may_load(mix_identity.as_bytes())?;
    match stored_bond {
        None => Ok(None),
        Some(stored_bond) => {
            let total_delegation =
                total_delegation_read(storage).may_load(mix_identity.as_bytes())?;
            Ok(Some(MixNodeBond {
                bond_amount: stored_bond.bond_amount,
                total_delegation: Coin {
                    denom: DENOM.to_owned(),
                    amount: total_delegation.unwrap_or_default(),
                },
                owner: stored_bond.owner,
                layer: stored_bond.layer,
                block_height: stored_bond.block_height,
                mix_node: stored_bond.mix_node,
                profit_margin_percent: stored_bond.profit_margin_percent,
            }))
        }
    }
}

// delegation related
pub fn all_mix_delegations_read<T>(storage: &dyn Storage) -> ReadonlyBucket<T>
where
    T: Serialize + DeserializeOwned,
{
    bucket_read(storage, PREFIX_MIX_DELEGATION)
}

pub fn mix_delegations<'a>(
    storage: &'a mut dyn Storage,
    mix_identity: IdentityKeyRef,
) -> Bucket<'a, RawDelegationData> {
    Bucket::multilevel(storage, &[PREFIX_MIX_DELEGATION, mix_identity.as_bytes()])
}

pub fn mix_delegations_read<'a>(
    storage: &'a dyn Storage,
    mix_identity: IdentityKeyRef,
) -> ReadonlyBucket<'a, RawDelegationData> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_MIX_DELEGATION, mix_identity.as_bytes()])
}

// TODO: note for JS when doing a deep review for the contract. Don't store it as (), instead do it as u8
pub fn reverse_mix_delegations<'a>(storage: &'a mut dyn Storage, owner: &Addr) -> Bucket<'a, ()> {
    Bucket::multilevel(storage, &[PREFIX_REVERSE_MIX_DELEGATION, owner.as_bytes()])
}

pub fn reverse_mix_delegations_read<'a>(
    storage: &'a dyn Storage,
    owner: &Addr,
) -> ReadonlyBucket<'a, ()> {
    ReadonlyBucket::multilevel(storage, &[PREFIX_REVERSE_MIX_DELEGATION, owner.as_bytes()])
}

#[cfg(test)]
mod tests {
    use super::super::storage;
    use super::*;
    use crate::helpers::identity_and_owner_to_bytes;
    use crate::support::tests::test_helpers;
    use config::defaults::DENOM;
    use cosmwasm_std::testing::{mock_dependencies, MockStorage};
    use cosmwasm_std::{coin, Addr, Uint128};
    use mixnet_contract::Layer;
    use mixnet_contract::MixNode;
    use mixnet_contract::MixNodeBond;
    use mixnet_contract::RawDelegationData;
    use mixnet_contract::{Gateway, IdentityKey};

    #[test]
    fn mixnode_single_read_retrieval() {
        let mut storage = MockStorage::new();
        let bond1 = test_helpers::stored_mixnode_bond_fixture();
        let bond2 = test_helpers::stored_mixnode_bond_fixture();
        mixnodes(&mut storage).save(b"bond1", &bond1).unwrap();
        mixnodes(&mut storage).save(b"bond2", &bond2).unwrap();

        let res1 = storage::mixnodes_read(&storage).load(b"bond1").unwrap();
        let res2 = storage::mixnodes_read(&storage).load(b"bond2").unwrap();
        assert_eq!(bond1, res1);
        assert_eq!(bond2, res2);
    }

    #[test]
    fn reading_mixnode_bond() {
        let mut storage = MockStorage::new();
        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        // produces an error if target mixnode doesn't exist
        let res = storage::read_mixnode_bond(&storage, node_owner.as_bytes());
        assert!(res.is_err());

        // returns appropriate value otherwise
        let bond_value = 1000;

        let mixnode_bond = StoredMixnodeBond {
            bond_amount: coin(bond_value, DENOM),
            owner: node_owner.clone(),
            layer: Layer::One,
            block_height: 12_345,
            mix_node: MixNode {
                identity_key: node_identity.clone(),
                ..test_helpers::mix_node_fixture()
            },
            profit_margin_percent: Some(10),
        };

        storage::mixnodes(&mut storage)
            .save(node_identity.as_bytes(), &mixnode_bond)
            .unwrap();

        assert_eq!(
            Uint128(bond_value),
            read_mixnode_bond_amount(&storage, node_identity.as_bytes()).unwrap()
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

        storage::mix_delegations(&mut deps.storage, &node_identity1)
            .save(delegation_owner1.as_bytes(), &raw_delegation1)
            .unwrap();
        storage::mix_delegations(&mut deps.storage, &node_identity2)
            .save(delegation_owner2.as_bytes(), &raw_delegation2)
            .unwrap();

        let res1 = storage::all_mix_delegations_read::<RawDelegationData>(&deps.storage)
            .load(&*test_helpers::identity_and_owner_to_bytes(
                &node_identity1,
                &delegation_owner1,
            ))
            .unwrap();
        let res2 = storage::all_mix_delegations_read::<RawDelegationData>(&deps.storage)
            .load(&*test_helpers::identity_and_owner_to_bytes(
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
        use crate::mixnodes::bonding_queries::query_mixnode_delegations_paged;
        use crate::rewards::transactions::MINIMUM_BLOCK_AGE_FOR_REWARDING;
        use cosmwasm_std::testing::mock_dependencies;
        use cosmwasm_std::Decimal;

        #[test]
        fn when_there_are_no_delegations() {
            let mut deps = mock_dependencies(&[]);
            let node_identity: IdentityKey = "nodeidentity".into();

            // 0.001
            let reward = Decimal::from_ratio(1u128, 1000u128);

            let total_increase = storage::increase_mix_delegated_stakes(
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
            storage::mix_delegations(&mut deps.storage, &node_identity)
                .save(
                    delegator_address.as_bytes(),
                    &RawDelegationData::new(1000u128.into(), delegation_blockstamp),
                )
                .unwrap();

            let total_increase = storage::increase_mix_delegated_stakes(
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
                storage::mix_delegations_read(&deps.storage, &node_identity)
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
            storage::mix_delegations(&mut deps.storage, &node_identity)
                .save(
                    delegator_address.as_bytes(),
                    &RawDelegationData::new(1000u128.into(), delegation_blockstamp),
                )
                .unwrap();

            let total_increase = storage::increase_mix_delegated_stakes(
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
                storage::mix_delegations_read(&deps.storage, &node_identity)
                    .load(delegator_address.as_bytes())
                    .unwrap()
            );

            let total_increase = storage::increase_mix_delegated_stakes(
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
                storage::mix_delegations_read(&deps.storage, &node_identity)
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
                storage::mix_delegations(&mut deps.storage, &node_identity)
                    .save(
                        delegator_address.as_bytes(),
                        &RawDelegationData::new(1000u128.into(), delegation_blockstamp),
                    )
                    .unwrap();
            }

            let total_increase = storage::increase_mix_delegated_stakes(
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
                    test_helpers::raw_delegation_fixture(1001),
                    storage::mix_delegations_read(&deps.storage, &node_identity)
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

            for i in 0..DELEGATION_PAGE_MAX_LIMIT * 10 {
                let delegator_address = Addr::unchecked(format!("address{}", i));
                storage::mix_delegations(&mut deps.storage, &node_identity)
                    .save(
                        delegator_address.as_bytes(),
                        &RawDelegationData::new(1000u128.into(), delegation_blockstamp),
                    )
                    .unwrap();
            }

            let total_increase = storage::increase_mix_delegated_stakes(
                &mut deps.storage,
                node_identity.as_ref(),
                reward,
                delegation_blockstamp + 2 * MINIMUM_BLOCK_AGE_FOR_REWARDING,
            )
            .unwrap();

            assert_eq!(
                Uint128(DELEGATION_PAGE_MAX_LIMIT as u128 * 10),
                total_increase
            );

            for i in 0..DELEGATION_PAGE_MAX_LIMIT * 10 {
                let delegator_address = Addr::unchecked(format!("address{}", i));
                assert_eq!(
                    test_helpers::raw_delegation_fixture(1001),
                    storage::mix_delegations_read(&deps.storage, &node_identity)
                        .load(delegator_address.as_bytes())
                        .unwrap()
                )
            }
        }
    }

    #[cfg(test)]
    mod reverse_mix_delegations {
        use super::*;
        use crate::support::tests::test_helpers;

        #[test]
        fn reverse_mix_delegation_exists() {
            let mut deps = test_helpers::init_contract();
            let node_identity: IdentityKey = "foo".into();
            let delegation_owner = Addr::unchecked("bar");

            storage::reverse_mix_delegations(&mut deps.storage, &delegation_owner)
                .save(node_identity.as_bytes(), &())
                .unwrap();

            assert!(storage::reverse_mix_delegations_read(
                deps.as_ref().storage,
                &delegation_owner
            )
            .may_load(node_identity.as_bytes())
            .unwrap()
            .is_some(),);
        }

        #[test]
        fn reverse_mix_delegation_returns_none_if_delegation_doesnt_exist() {
            let mut deps = test_helpers::init_contract();

            let node_identity1: IdentityKey = "foo1".into();
            let node_identity2: IdentityKey = "foo2".into();
            let delegation_owner1 = Addr::unchecked("bar");
            let delegation_owner2 = Addr::unchecked("bar2");

            assert!(storage::reverse_mix_delegations_read(
                deps.as_ref().storage,
                &delegation_owner1
            )
            .may_load(node_identity1.as_bytes())
            .unwrap()
            .is_none());

            // add delegation for a different node
            storage::reverse_mix_delegations(&mut deps.storage, &delegation_owner1)
                .save(node_identity2.as_bytes(), &())
                .unwrap();

            assert!(storage::reverse_mix_delegations_read(
                deps.as_ref().storage,
                &delegation_owner1
            )
            .may_load(node_identity1.as_bytes())
            .unwrap()
            .is_none());

            // add delegation from a different owner
            storage::reverse_mix_delegations(&mut deps.storage, &delegation_owner2)
                .save(node_identity1.as_bytes(), &())
                .unwrap();

            assert!(storage::reverse_mix_delegations_read(
                deps.as_ref().storage,
                &delegation_owner1
            )
            .may_load(node_identity1.as_bytes())
            .unwrap()
            .is_none());
        }
    }
}

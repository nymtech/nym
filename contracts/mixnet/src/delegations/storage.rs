// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Storage;
use cosmwasm_storage::{bucket_read, Bucket, ReadonlyBucket};
use mixnet_contract::{Addr, IdentityKeyRef, RawDelegationData};
use serde::de::DeserializeOwned;
use serde::Serialize;

// storage prefixes
const PREFIX_MIX_DELEGATION: &[u8] = b"md";
const PREFIX_REVERSE_MIX_DELEGATION: &[u8] = b"dm";

// paged retrieval limits for all queries and transactions
// currently the maximum limit before running into memory issue is somewhere between 1150 and 1200
pub(crate) const DELEGATION_PAGE_MAX_LIMIT: u32 = 500;
pub(crate) const DELEGATION_PAGE_DEFAULT_LIMIT: u32 = 250;

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
    use crate::delegations::storage;
    use crate::support::tests::test_helpers;
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::Addr;
    use mixnet_contract::IdentityKey;
    use mixnet_contract::RawDelegationData;

    #[test]
    fn all_mixnode_delegations_read_retrieval() {
        let mut deps = mock_dependencies();
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

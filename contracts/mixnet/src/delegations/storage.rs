// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};
use mixnet_contract::{Addr, Delegation, IdentityKey};

// storage prefixes
const DELEGATION_PK_NAMESPACE: &str = "dl";
const DELEGATION_OWNER_IDX_NAMESPACE: &str = "dlo";
const DELEGATION_MIXNODE_IDX_NAMESPACE: &str = "dlm";

// paged retrieval limits for all queries and transactions
pub(crate) const DELEGATION_PAGE_MAX_LIMIT: u32 = 500;
pub(crate) const DELEGATION_PAGE_DEFAULT_LIMIT: u32 = 250;

// It's a composite key on node's identity and delegator address
type PrimaryKey = Vec<u8>;

pub(crate) struct DelegationIndex<'a> {
    pub(crate) owner: MultiIndex<'a, (Addr, PrimaryKey), Delegation>,

    pub(crate) mixnode: MultiIndex<'a, (IdentityKey, PrimaryKey), Delegation>,
}

impl<'a> IndexList<Delegation> for DelegationIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Delegation>> + '_> {
        let v: Vec<&dyn Index<Delegation>> = vec![&self.owner, &self.mixnode];
        Box::new(v.into_iter())
    }
}

// I was really going back and forth about the data stored on the disk vs primary key duplication.
// It was basically between convenience and bloat, but in the end I decided the convenience wins.
//
// Basically I had 2 approaches. a) store delegator address and mixnode identity only as primary key of delegation or
// b) store it both as primary key AND inside delegation data.
// For the longest time I was in favour of a), since that removed any data duplication. However...,
// that also required that during index creation I recovered delegator address and mixnode identity
// from the Vec<u8>. That doesn't sound that terrible. However, even though I'm 99.99% certain that
// conversion would be impossible to fail, I'd still have to call an `unwrap` here due to required
// type signature and I didn't feel super comfortable doing that in our smart contract...
// So to get rid of this uncertainty I went with the b) approach. Even though each stored delegation
// takes over ~250B (since the key has to be duplicated), in the grand blockchain scheme of things
// it's not that terrible. Say we had 100_000_000 delegations -> that's still only 25GB of data
// and as a nice by-product it cleans up code a little bit by only having a single Delegation type.
pub(crate) fn delegations<'a>() -> IndexedMap<'a, PrimaryKey, Delegation, DelegationIndex<'a>> {
    let indexes = DelegationIndex {
        owner: MultiIndex::new(
            |d, pk| (d.owner.clone(), pk),
            DELEGATION_PK_NAMESPACE,
            DELEGATION_OWNER_IDX_NAMESPACE,
        ),
        mixnode: MultiIndex::new(
            |d, pk| (d.node_identity.clone(), pk),
            DELEGATION_PK_NAMESPACE,
            DELEGATION_MIXNODE_IDX_NAMESPACE,
        ),
    };

    IndexedMap::new(DELEGATION_PK_NAMESPACE, indexes)
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

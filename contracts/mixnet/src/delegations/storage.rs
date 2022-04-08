// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cw_storage_plus::{Index, IndexList, IndexedMap, Map, MultiIndex};
use mixnet_contract_common::{mixnode::DelegationEvent, Addr, Delegation, IdentityKey};

// storage prefixes
pub const DELEGATION_PK_NAMESPACE: &str = "dl";
pub const DELEGATION_OWNER_IDX_NAMESPACE: &str = "dlo";
pub const DELEGATION_MIXNODE_IDX_NAMESPACE: &str = "dlm";

pub const PENDING_DELEGATION_EVENTS: Map<
    (OwnerAddress, BlockHeight, IdentityKey),
    DelegationEvent,
> = Map::new("pend2");

// paged retrieval limits for all queries and transactions
pub(crate) const DELEGATION_PAGE_MAX_LIMIT: u32 = 500;
pub(crate) const DELEGATION_PAGE_DEFAULT_LIMIT: u32 = 250;

type BlockHeight = u64;
type OwnerAddress = Vec<u8>;
// It's a composite key on node's identity, delegator address, and block height
type PrimaryKey = (IdentityKey, OwnerAddress, BlockHeight);

pub(crate) struct DelegationIndex<'a> {
    pub(crate) owner: MultiIndex<'a, Addr, Delegation, PrimaryKey>,

    pub(crate) mixnode: MultiIndex<'a, IdentityKey, Delegation, PrimaryKey>,
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
            |d| d.owner.clone(),
            DELEGATION_PK_NAMESPACE,
            DELEGATION_OWNER_IDX_NAMESPACE,
        ),
        mixnode: MultiIndex::new(
            |d| d.node_identity.clone(),
            DELEGATION_PK_NAMESPACE,
            DELEGATION_MIXNODE_IDX_NAMESPACE,
        ),
    };

    IndexedMap::new(DELEGATION_PK_NAMESPACE, indexes)
}

#[cfg(test)]
mod tests {
    use crate::delegations::storage;
    use cosmwasm_std::Addr;
    use mixnet_contract_common::IdentityKey;

    #[cfg(test)]
    mod reverse_mix_delegations {
        use super::*;
        use crate::support::tests::test_helpers;
        use config::defaults::DENOM;
        use cosmwasm_std::testing::mock_env;
        use cosmwasm_std::{coin, Order};
        use mixnet_contract_common::Delegation;

        #[test]
        fn reverse_mix_delegation_exists() {
            let mut deps = test_helpers::init_contract();
            let node_identity: IdentityKey = "foo".into();
            let delegation_owner = Addr::unchecked("bar");
            let delegation = coin(12345, DENOM);

            let dummy_data = Delegation::new(
                delegation_owner.clone(),
                node_identity.clone(),
                delegation,
                mock_env().block.height,
                None,
            );

            storage::delegations()
                .save(
                    &mut deps.storage,
                    (node_identity, delegation_owner.as_bytes().to_vec(), 0),
                    &dummy_data,
                )
                .unwrap();

            let read = storage::delegations()
                .idx
                .owner
                .prefix(delegation_owner)
                .range(&deps.storage, None, None, Order::Ascending)
                .map(|record| record.unwrap().1)
                .collect::<Vec<_>>();

            assert_eq!(1, read.len());
            assert_eq!(dummy_data, read[0]);
        }

        #[test]
        fn reverse_mix_delegation_returns_none_if_delegation_doesnt_exist() {
            let mut deps = test_helpers::init_contract();

            let node_identity1: IdentityKey = "foo1".into();
            let node_identity2: IdentityKey = "foo2".into();
            let delegation_owner1 = Addr::unchecked("bar");
            let delegation_owner2 = Addr::unchecked("bar2");
            let delegation = coin(12345, DENOM);

            assert!(test_helpers::read_delegation(
                deps.as_ref().storage,
                &node_identity1,
                delegation_owner1.as_bytes(),
                mock_env().block.height
            )
            .is_none());

            // add delegation for a different node
            let dummy_data = Delegation::new(
                delegation_owner1.clone(),
                node_identity2,
                delegation.clone(),
                mock_env().block.height,
                None,
            );
            storage::delegations()
                .save(
                    &mut deps.storage,
                    (
                        node_identity1.clone(),
                        delegation_owner1.as_bytes().to_vec(),
                        0,
                    ),
                    &dummy_data,
                )
                .unwrap();

            storage::delegations()
                .idx
                .owner
                .prefix(delegation_owner1.clone())
                .range(&deps.storage, None, None, Order::Ascending)
                .map(|record| record.unwrap().1)
                .for_each(|delegation| assert_ne!(delegation.node_identity, node_identity1));

            // add delegation from a different owner
            let dummy_data = Delegation::new(
                delegation_owner2.clone(),
                node_identity1.clone(),
                delegation,
                mock_env().block.height,
                None,
            );
            storage::delegations()
                .save(
                    &mut deps.storage,
                    (
                        node_identity1.clone(),
                        delegation_owner2.as_bytes().to_vec(),
                        0,
                    ),
                    &dummy_data,
                )
                .unwrap();

            storage::delegations()
                .idx
                .owner
                .prefix(delegation_owner1.clone())
                .range(&deps.storage, None, None, Order::Ascending)
                .map(|record| record.unwrap().1)
                .for_each(|delegation| assert_ne!(delegation.node_identity, node_identity1));
        }
    }
}

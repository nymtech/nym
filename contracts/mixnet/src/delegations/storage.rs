// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{
    DELEGATION_MIXNODE_IDX_NAMESPACE, DELEGATION_OWNER_IDX_NAMESPACE, DELEGATION_PK_NAMESPACE,
};
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};
use mixnet_contract_common::delegation::OwnerProxySubKey;
use mixnet_contract_common::{Addr, Delegation, NodeId};

// type BlockHeight = u64;

// It's a composite key on node's id and delegator address
// type PrimaryKey = (IdentityKey, OwnerAddress, BlockHeight);
type PrimaryKey = (NodeId, OwnerProxySubKey);

pub(crate) struct DelegationIndex<'a> {
    pub(crate) owner: MultiIndex<'a, Addr, Delegation, PrimaryKey>,

    pub(crate) mixnode: MultiIndex<'a, NodeId, Delegation, PrimaryKey>,
}

impl<'a> IndexList<Delegation> for DelegationIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Delegation>> + '_> {
        let v: Vec<&dyn Index<Delegation>> = vec![&self.owner, &self.mixnode];
        Box::new(v.into_iter())
    }
}

pub(crate) fn delegations<'a>() -> IndexedMap<'a, PrimaryKey, Delegation, DelegationIndex<'a>> {
    let indexes = DelegationIndex {
        owner: MultiIndex::new(
            |d| d.owner.clone(),
            DELEGATION_PK_NAMESPACE,
            DELEGATION_OWNER_IDX_NAMESPACE,
        ),
        mixnode: MultiIndex::new(
            |d| d.node_id,
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

    // #[cfg(test)]
    // mod reverse_mix_delegations {
    //     use super::*;
    //     use crate::support::tests::test_helpers;
    //     use config::defaults::MIX_DENOM;
    //     use cosmwasm_std::testing::mock_env;
    //     use cosmwasm_std::{coin, Order};
    //     use mixnet_contract_common::Delegation;
    //
    //     #[test]
    //     fn reverse_mix_delegation_exists() {
    //         let mut deps = test_helpers::init_contract();
    //         let node_id = 42;
    //         let period = 123;
    //         let delegation_owner = Addr::unchecked("bar");
    //         let delegation = coin(12345, MIX_DENOM.base);
    //
    //         let dummy_data = Delegation::new(
    //             delegation_owner.clone(),
    //             node_id,
    //             period,
    //             delegation,
    //             mock_env().block.height,
    //             None,
    //         );
    //
    //         storage::delegations()
    //             .save(&mut deps.storage, dummy_data.storage_key(), &dummy_data)
    //             .unwrap();
    //
    //         let read = storage::delegations()
    //             .idx
    //             .owner
    //             .prefix(delegation_owner)
    //             .range(&deps.storage, None, None, Order::Ascending)
    //             .map(|record| record.unwrap().1)
    //             .collect::<Vec<_>>();
    //
    //         assert_eq!(1, read.len());
    //         assert_eq!(dummy_data, read[0]);
    //     }
    //
    //     #[test]
    //     fn reverse_mix_delegation_returns_none_if_delegation_doesnt_exist() {
    //         let mut deps = test_helpers::init_contract();
    //
    //         let node_id1 = 1;
    //         let node_id2 = 2;
    //         let delegation_owner1 = Addr::unchecked("bar");
    //         let delegation_owner2 = Addr::unchecked("bar2");
    //         let delegation = coin(12345, MIX_DENOM.base);
    //
    //         assert!(test_helpers::read_delegation(
    //             deps.as_ref().storage,
    //             node_id1,
    //             delegation_owner1.as_bytes(),
    //         )
    //         .is_none());
    //
    //         // add delegation for a different node
    //         let dummy_data = Delegation::new(
    //             delegation_owner1.clone(),
    //             node_id2,
    //             42,
    //             delegation.clone(),
    //             mock_env().block.height,
    //             None,
    //         );
    //         storage::delegations()
    //             .save(&mut deps.storage, dummy_data.storage_key(), &dummy_data)
    //             .unwrap();
    //
    //         storage::delegations()
    //             .idx
    //             .owner
    //             .prefix(delegation_owner1.clone())
    //             .range(&deps.storage, None, None, Order::Ascending)
    //             .map(|record| record.unwrap().1)
    //             .for_each(|delegation| assert_ne!(delegation.node_id, node_id1));
    //
    //         // add delegation from a different owner
    //         let dummy_data = Delegation::new(
    //             delegation_owner2,
    //             node_id1,
    //             42,
    //             delegation,
    //             mock_env().block.height,
    //             None,
    //         );
    //         storage::delegations()
    //             .save(&mut deps.storage, dummy_data.storage_key(), &dummy_data)
    //             .unwrap();
    //
    //         storage::delegations()
    //             .idx
    //             .owner
    //             .prefix(delegation_owner1)
    //             .range(&deps.storage, None, None, Order::Ascending)
    //             .map(|record| record.unwrap().1)
    //             .for_each(|delegation| assert_ne!(delegation.node_id, node_id1));
    //     }
    // }
}

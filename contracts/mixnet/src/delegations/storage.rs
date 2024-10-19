// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{
    DELEGATION_MIXNODE_IDX_NAMESPACE, DELEGATION_OWNER_IDX_NAMESPACE, DELEGATION_PK_NAMESPACE,
};
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};
use mixnet_contract_common::delegation::OwnerProxySubKey;
use mixnet_contract_common::{Addr, Delegation, NodeId};

// It's a composite key on node's id and delegator address
type PrimaryKey = (NodeId, OwnerProxySubKey);

pub(crate) struct DelegationIndex<'a> {
    pub(crate) owner: MultiIndex<'a, Addr, Delegation, PrimaryKey>,

    pub(crate) mixnode: MultiIndex<'a, NodeId, Delegation, PrimaryKey>,
}

impl IndexList<Delegation> for DelegationIndex<'_> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Delegation>> + '_> {
        let v: Vec<&dyn Index<Delegation>> = vec![&self.owner, &self.mixnode];
        Box::new(v.into_iter())
    }
}

pub(crate) fn delegations<'a>() -> IndexedMap<'a, PrimaryKey, Delegation, DelegationIndex<'a>> {
    let indexes = DelegationIndex {
        owner: MultiIndex::new(
            |_pk, d| d.owner.clone(),
            DELEGATION_PK_NAMESPACE,
            DELEGATION_OWNER_IDX_NAMESPACE,
        ),
        mixnode: MultiIndex::new(
            |_pk, d| d.node_id,
            DELEGATION_PK_NAMESPACE,
            DELEGATION_MIXNODE_IDX_NAMESPACE,
        ),
    };

    IndexedMap::new(DELEGATION_PK_NAMESPACE, indexes)
}

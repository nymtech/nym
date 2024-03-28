// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{
    MIXNODES_IDENTITY_IDX_NAMESPACE, MIXNODES_OWNER_IDX_NAMESPACE, MIXNODES_PK_NAMESPACE,
    MIXNODES_SPHINX_IDX_NAMESPACE, PENDING_MIXNODE_CHANGES_NAMESPACE,
    UNBONDED_MIXNODES_IDENTITY_IDX_NAMESPACE, UNBONDED_MIXNODES_OWNER_IDX_NAMESPACE,
    UNBONDED_MIXNODES_PK_NAMESPACE,
};
use cw_storage_plus::{Index, IndexList, IndexedMap, Map, MultiIndex, UniqueIndex};
use mixnet_contract_common::mixnode::{PendingMixNodeChanges, UnbondedMixnode};
use mixnet_contract_common::SphinxKey;
use mixnet_contract_common::{Addr, IdentityKey, MixNodeBond, NodeId};

pub const PENDING_MIXNODE_CHANGES: Map<NodeId, PendingMixNodeChanges> =
    Map::new(PENDING_MIXNODE_CHANGES_NAMESPACE);

// keeps track of `node_id -> IdentityKey, Owner, unbonding_height` so we'd known a bit more about past mixnodes
// if we ever decide it's too bloaty, we can deprecate it and start removing all data in
// subsequent migrations
pub(crate) struct UnbondedMixnodeIndex<'a> {
    pub(crate) owner: MultiIndex<'a, Addr, UnbondedMixnode, NodeId>,

    pub(crate) identity_key: MultiIndex<'a, IdentityKey, UnbondedMixnode, NodeId>,
}

impl<'a> IndexList<UnbondedMixnode> for UnbondedMixnodeIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<UnbondedMixnode>> + '_> {
        let v: Vec<&dyn Index<UnbondedMixnode>> = vec![&self.owner, &self.identity_key];
        Box::new(v.into_iter())
    }
}

pub(crate) fn unbonded_mixnodes<'a>(
) -> IndexedMap<'a, NodeId, UnbondedMixnode, UnbondedMixnodeIndex<'a>> {
    let indexes = UnbondedMixnodeIndex {
        owner: MultiIndex::new(
            |_pk, d| d.owner.clone(),
            UNBONDED_MIXNODES_PK_NAMESPACE,
            UNBONDED_MIXNODES_OWNER_IDX_NAMESPACE,
        ),
        identity_key: MultiIndex::new(
            |_pk, d| d.identity_key.clone(),
            UNBONDED_MIXNODES_PK_NAMESPACE,
            UNBONDED_MIXNODES_IDENTITY_IDX_NAMESPACE,
        ),
    };
    IndexedMap::new(UNBONDED_MIXNODES_PK_NAMESPACE, indexes)
}

pub(crate) struct MixnodeBondIndex<'a> {
    pub(crate) owner: UniqueIndex<'a, Addr, MixNodeBond>,

    pub(crate) identity_key: UniqueIndex<'a, IdentityKey, MixNodeBond>,

    pub(crate) sphinx_key: UniqueIndex<'a, SphinxKey, MixNodeBond>,
}

// IndexList is just boilerplate code for fetching a struct's indexes
// note that from my understanding this will be converted into a macro at some point in the future
impl<'a> IndexList<MixNodeBond> for MixnodeBondIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<MixNodeBond>> + '_> {
        let v: Vec<&dyn Index<MixNodeBond>> =
            vec![&self.owner, &self.identity_key, &self.sphinx_key];
        Box::new(v.into_iter())
    }
}

// mixnode_bonds() is the storage access function.
pub(crate) fn mixnode_bonds<'a>() -> IndexedMap<'a, NodeId, MixNodeBond, MixnodeBondIndex<'a>> {
    let indexes = MixnodeBondIndex {
        owner: UniqueIndex::new(|d| d.owner.clone(), MIXNODES_OWNER_IDX_NAMESPACE),
        identity_key: UniqueIndex::new(
            |d| d.mix_node.identity_key.clone(),
            MIXNODES_IDENTITY_IDX_NAMESPACE,
        ),
        sphinx_key: UniqueIndex::new(
            |d| d.mix_node.sphinx_key.clone(),
            MIXNODES_SPHINX_IDX_NAMESPACE,
        ),
    };
    IndexedMap::new(MIXNODES_PK_NAMESPACE, indexes)
}

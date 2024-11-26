// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{
    NODE_ID_COUNTER_KEY, NYMNODE_ACTIVE_ROLE_ASSIGNMENT_KEY, NYMNODE_IDENTITY_IDX_NAMESPACE,
    NYMNODE_OWNER_IDX_NAMESPACE, NYMNODE_PK_NAMESPACE, NYMNODE_REWARDED_SET_METADATA_NAMESPACE,
    NYMNODE_ROLES_ASSIGNMENT_NAMESPACE, PENDING_NYMNODE_CHANGES_NAMESPACE,
    UNBONDED_NYMNODE_IDENTITY_IDX_NAMESPACE, UNBONDED_NYMNODE_OWNER_IDX_NAMESPACE,
    UNBONDED_NYMNODE_PK_NAMESPACE,
};
use crate::nodes::storage::helpers::RoleStorageBucket;
use cosmwasm_std::Addr;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex, UniqueIndex};
use mixnet_contract_common::nym_node::{NymNodeBond, RewardedSetMetadata, Role, UnbondedNymNode};
use mixnet_contract_common::{NodeId, PendingNodeChanges};
use nym_contracts_common::IdentityKey;

pub(crate) mod helpers;

pub(crate) use helpers::*;

// IMPORTANT NOTE: we're using the same storage key as we had for MIXNODE_ID_COUNTER,
// so that we could start from the old values
pub const NYMNODE_ID_COUNTER: Item<NodeId> = Item::new(NODE_ID_COUNTER_KEY);

// each nym-node has 3 storage buckets:
// - `NymNodeBondIndex` to keep track of the actual node information
// - `PENDING_NYMNODE_CHANGES` to keep track of current params/pledge changes
// - `rewards_storage::NYMNODE_REWARDING` to keep track of data needed for reward calculation

pub const PENDING_NYMNODE_CHANGES: Map<NodeId, PendingNodeChanges> =
    Map::new(PENDING_NYMNODE_CHANGES_NAMESPACE);

pub mod rewarded_set {
    use super::*;

    // role assignment period is an awkward time for querying for up-to-date data
    // for example if we have assigned layer1 and layer2 but not yet touched layer3,
    // the state would be inconsistent since it'd have data of layer3 from previous epoch
    //
    // thus we just toggle the virtual pointer between 2 buckets
    // since we also don't want to keep state for all epochs.
    //
    // general rule of thumb: we're always READING from the active bucket,
    // but we're WRITING to the inactive bucket (because it's still being built)
    /// Item keeping track of the current active node assignment
    pub const ACTIVE_ROLES_BUCKET: Item<RoleStorageBucket> =
        Item::new(NYMNODE_ACTIVE_ROLE_ASSIGNMENT_KEY);

    // NOTES FOR FUTURE IMPLEMENTATION:
    // to implement pre-announcement of nodes, you don't have to do much. literally almost nothing at all,
    // you'd just have to expose the current inactive bucket and make sure to correctly invalidate it when being written to

    // it feels more efficient to have a single bulk read/write operation per role
    // as opposed to storing everything under separate keys.
    // however, the drawback is a potentially huge writing cost, but I don't think
    // we're going to have 1k+ nodes per layer any time soon for it to be a problem
    //
    // note: the actual resolution of which node id corresponds to which ip/identity
    // is left to up the caller
    //
    // storage note: we use `u8` rather than `RoleStorageBucket` in the composite key
    // to avoid having to derive all required traits
    /// Storage map of `(RoleStorageBucket, Role)` => set of nodes with that assigned role
    pub const ROLES: Map<(u8, Role), Vec<NodeId>> = Map::new(NYMNODE_ROLES_ASSIGNMENT_NAMESPACE);

    /// Storage map of metadata associated with particular `RoleStorageBucket`
    pub const ROLES_METADATA: Map<u8, RewardedSetMetadata> =
        Map::new(NYMNODE_REWARDED_SET_METADATA_NAMESPACE);
}

pub(crate) struct NymNodeBondIndex<'a> {
    pub(crate) owner: UniqueIndex<'a, Addr, NymNodeBond>,

    pub(crate) identity_key: UniqueIndex<'a, IdentityKey, NymNodeBond>,
}

impl IndexList<NymNodeBond> for NymNodeBondIndex<'_> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<NymNodeBond>> + '_> {
        let v: Vec<&dyn Index<NymNodeBond>> = vec![&self.owner, &self.identity_key];
        Box::new(v.into_iter())
    }
}

// nym_nodes() is the storage access function.
pub(crate) fn nym_nodes<'a>() -> IndexedMap<'a, NodeId, NymNodeBond, NymNodeBondIndex<'a>> {
    let indexes = NymNodeBondIndex {
        owner: UniqueIndex::new(|d| d.owner.clone(), NYMNODE_OWNER_IDX_NAMESPACE),
        identity_key: UniqueIndex::new(
            |d| d.node.identity_key.clone(),
            NYMNODE_IDENTITY_IDX_NAMESPACE,
        ),
    };
    IndexedMap::new(NYMNODE_PK_NAMESPACE, indexes)
}

// keeps track of `node_id -> IdentityKey, Owner, unbonding_height` so we'd known a bit more about past nodes
// if we ever decide it's too bloaty, we can deprecate it and start removing all data in
// subsequent migrations
pub(crate) struct UnbondedNymNodeIndex<'a> {
    pub(crate) owner: MultiIndex<'a, Addr, UnbondedNymNode, NodeId>,

    pub(crate) identity_key: MultiIndex<'a, IdentityKey, UnbondedNymNode, NodeId>,
}

impl IndexList<UnbondedNymNode> for UnbondedNymNodeIndex<'_> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<UnbondedNymNode>> + '_> {
        let v: Vec<&dyn Index<UnbondedNymNode>> = vec![&self.owner, &self.identity_key];
        Box::new(v.into_iter())
    }
}

pub(crate) fn unbonded_nym_nodes<'a>(
) -> IndexedMap<'a, NodeId, UnbondedNymNode, UnbondedNymNodeIndex<'a>> {
    let indexes = UnbondedNymNodeIndex {
        owner: MultiIndex::new(
            |_pk, d| d.owner.clone(),
            UNBONDED_NYMNODE_PK_NAMESPACE,
            UNBONDED_NYMNODE_OWNER_IDX_NAMESPACE,
        ),
        identity_key: MultiIndex::new(
            |_pk, d| d.identity_key.clone(),
            UNBONDED_NYMNODE_PK_NAMESPACE,
            UNBONDED_NYMNODE_IDENTITY_IDX_NAMESPACE,
        ),
    };
    IndexedMap::new(UNBONDED_NYMNODE_PK_NAMESPACE, indexes)
}

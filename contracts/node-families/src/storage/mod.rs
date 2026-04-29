// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::storage_indexes::{
    NodeFamiliesIndex, NodeFamilyInvitationIndex, PastFamilyInvitationsIndex,
    PastFamilyMembersIndex,
};
use cosmwasm_std::{Addr, Storage};
use cw_controllers::Admin;
use cw_storage_plus::{IndexedMap, Item, Map};
use node_families_contract_common::constants::storage_keys;
use node_families_contract_common::{
    FamilyInvitation, NodeFamiliesContractError, NodeFamily, NodeFamilyId, PastFamilyInvitation,
    PastFamilyMember,
};
use nym_mixnet_contract_common::NodeId;

mod storage_indexes;

/// Composite primary key for the invitation / past-member maps:
/// `(family id, node id)`. Only one pending invitation can exist for a given
/// `(family, node)` pair at a time.
pub(crate) type FamilyMember = (NodeFamilyId, NodeId);

/// Container for every storage handle used by the contract.
///
/// Constructed once via [`NodeFamiliesStorage::new`] and accessed through a
/// `lazy_static`-style singleton in the entry point modules.
pub struct NodeFamiliesStorage<'a> {
    /// Admin of the contract; gates privileged operations.
    pub(crate) contract_admin: Admin,

    /// Address of the mixnet contract; used to verify a node id refers to a
    /// real, registered node.
    pub(crate) mixnet_contract_address: Item<Addr>,

    /// Monotonically increasing id assigned to every newly created family.
    /// Ids start at `1` (see [`NodeFamiliesStorage::next_family_id`]); `0` is
    /// reserved as a "no family" sentinel.
    pub(crate) node_family_id_counter: Item<NodeFamilyId>,

    /// All existing families, keyed by id, with a unique secondary index on
    /// `owner` enforcing the **one-family-per-owner-address** invariant.
    pub(crate) families: IndexedMap<NodeFamilyId, NodeFamily, NodeFamiliesIndex<'a>>,

    /// Mapping from a node id to the family it currently belongs to. A node
    /// belongs to at most one family at a time, so this is a plain `Map`.
    pub(crate) family_members: Map<NodeId, NodeFamilyId>,

    /// Currently outstanding family invitations, indexed by both family id
    /// and node id (a single node can simultaneously hold invitations from
    /// multiple families).
    pub(crate) pending_family_invitations:
        IndexedMap<FamilyMember, FamilyInvitation, NodeFamilyInvitationIndex<'a>>,

    // ##### historical data #####
    //
    // The two maps below archive terminal events. The trailing `u64` in the
    // composite key is a per-`(family, node)` counter — a node can be removed
    // from (or rejected by) the same family more than once, and we cannot use
    // the block timestamp to disambiguate because multiple txs may share a
    // block.
    /// Archive of family memberships that have ended (kicked, left, or family
    /// disbanded). Key: `((family_id, node_id), counter)`.
    pub(crate) past_family_members:
        IndexedMap<(FamilyMember, u64), PastFamilyMember, PastFamilyMembersIndex<'a>>,

    /// Archive of invitations that reached a terminal `Rejected` / `Revoked`
    /// state. Timed-out invitations are **not** archived here — there is no
    /// background process that sweeps expired entries out of
    /// [`Self::pending_family_invitations`].
    pub(crate) past_family_invitations:
        IndexedMap<(FamilyMember, u64), PastFamilyInvitation, PastFamilyInvitationsIndex<'a>>,
}

impl<'a> NodeFamiliesStorage<'a> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        NodeFamiliesStorage {
            contract_admin: Admin::new(storage_keys::CONTRACT_ADMIN),
            mixnet_contract_address: Item::new(storage_keys::MIXNET_CONTRACT_ADDRESS),
            node_family_id_counter: Item::new(storage_keys::NODE_FAMILY_ID_COUNTER),
            families: IndexedMap::new(storage_keys::FAMILIES_NAMESPACE, NodeFamiliesIndex::new()),
            family_members: Map::new(storage_keys::NODE_FAMILY_MEMBERS),
            pending_family_invitations: IndexedMap::new(
                storage_keys::INVITATIONS_NAMESPACE,
                NodeFamilyInvitationIndex::new(),
            ),
            past_family_members: IndexedMap::new(
                storage_keys::PAST_FAMILY_MEMBER_NAMESPACE,
                PastFamilyMembersIndex::new(),
            ),
            past_family_invitations: IndexedMap::new(
                storage_keys::PAST_INVITATIONS_NAMESPACE,
                PastFamilyInvitationsIndex::new(),
            ),
        }
    }

    /// Allocate the next [`NodeFamilyId`] and persist the bumped counter.
    ///
    /// Ids are issued starting from `1`; `0` is reserved as a "no family"
    /// sentinel value and must never be assigned to a real family.
    pub(crate) fn next_family_id(
        &self,
        store: &mut dyn Storage,
    ) -> Result<NodeFamilyId, NodeFamiliesContractError> {
        let next_id = self
            .node_family_id_counter
            .may_load(store)?
            .unwrap_or_default()
            + 1;
        self.node_family_id_counter.save(store, &next_id)?;
        Ok(next_id)
    }
}

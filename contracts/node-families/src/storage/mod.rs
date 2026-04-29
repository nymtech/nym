// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// storage will be used in subsequent PRs/tickets
#![allow(dead_code)]

use crate::storage::storage_indexes::{
    NodeFamiliesIndex, NodeFamilyInvitationIndex, PastFamilyInvitationsIndex,
    PastFamilyMembersIndex,
};
use cosmwasm_std::{Addr, Env, Storage};
use cw_controllers::Admin;
use cw_storage_plus::{IndexedMap, Item, Map};
use node_families_contract_common::constants::storage_keys;
use node_families_contract_common::{
    FamilyInvitation, FamilyInvitationStatus, NodeFamiliesContractError, NodeFamily, NodeFamilyId,
    PastFamilyInvitation, PastFamilyMember,
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

    /// Per-`(family, node)` counter for the [`Self::past_family_members`]
    /// archive — yields the next free `counter` slot when archiving a new
    /// past-membership record. Stored explicitly (rather than derived via
    /// range scan) to keep archival writes O(1).
    pub(crate) past_family_member_counter: Map<FamilyMember, u64>,

    /// Archive of invitations that reached a terminal `Accepted` / `Rejected`
    /// / `Revoked` state. Timed-out invitations are **not** archived here —
    /// there is no background process that sweeps expired entries out of
    /// [`Self::pending_family_invitations`].
    pub(crate) past_family_invitations:
        IndexedMap<(FamilyMember, u64), PastFamilyInvitation, PastFamilyInvitationsIndex<'a>>,

    /// Per-`(family, node)` counter for the [`Self::past_family_invitations`]
    /// archive — yields the next free `counter` slot when archiving a
    /// terminal invitation event.
    pub(crate) past_family_invitation_counter: Map<FamilyMember, u64>,
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
            past_family_member_counter: Map::new(
                storage_keys::PAST_FAMILY_MEMBER_COUNTER_NAMESPACE,
            ),
            past_family_invitations: IndexedMap::new(
                storage_keys::PAST_INVITATIONS_NAMESPACE,
                PastFamilyInvitationsIndex::new(),
            ),
            past_family_invitation_counter: Map::new(
                storage_keys::PAST_INVITATIONS_COUNTER_NAMESPACE,
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

    /// Allocate the next free archive slot for the [`Self::past_family_invitations`]
    /// map under the given `(family, node)` key, and persist the bumped counter.
    ///
    /// Slots are issued starting from `0` and increase by 1 on every call.
    pub(crate) fn next_past_invitation_counter(
        &self,
        store: &mut dyn Storage,
        key: FamilyMember,
    ) -> Result<u64, NodeFamiliesContractError> {
        let counter = self
            .past_family_invitation_counter
            .may_load(store, key)?
            .unwrap_or_default();
        self.past_family_invitation_counter
            .save(store, key, &(counter + 1))?;
        Ok(counter)
    }

    /// Allocate the next free archive slot for the [`Self::past_family_members`]
    /// map under the given `(family, node)` key, and persist the bumped counter.
    ///
    /// Slots are issued starting from `0` and increase by 1 on every call.
    pub(crate) fn next_past_member_counter(
        &self,
        store: &mut dyn Storage,
        key: FamilyMember,
    ) -> Result<u64, NodeFamiliesContractError> {
        let counter = self
            .past_family_member_counter
            .may_load(store, key)?
            .unwrap_or_default();
        self.past_family_member_counter
            .save(store, key, &(counter + 1))?;
        Ok(counter)
    }

    /// Persist a brand-new family in storage.
    ///
    /// Assigns a fresh [`NodeFamilyId`], stamps `created_at` from `env`
    /// (unix seconds) and starts the membership counter at `0` — the owner
    /// is **not** counted as a member.
    ///
    /// The caller (a transaction handler) is responsible for:
    /// - validating `name`, `description` and `owner`;
    /// - ensuring `owner` does not already own a family **and** is not
    ///   currently a member of one.
    ///
    /// Returns the freshly persisted [`NodeFamily`]. The underlying
    /// `IndexedMap` enforces the one-family-per-owner invariant via the
    /// unique index on `owner` as a defence-in-depth check, so this call
    /// will fail if `owner` already owns a family — but the caller must not
    /// rely on it for the membership check.
    pub(crate) fn register_new_family(
        &self,
        store: &mut dyn Storage,
        env: &Env,
        owner: Addr,
        name: String,
        description: String,
    ) -> Result<NodeFamily, NodeFamiliesContractError> {
        let id = self.next_family_id(store)?;
        let family = NodeFamily {
            id,
            name,
            description,
            owner,
            members: 0,
            created_at: env.block.time.seconds(),
        };
        self.families.save(store, id, &family)?;
        Ok(family)
    }

    /// Accept a pending invitation for `node_id` to join `family_id`.
    ///
    /// Performs the full storage transition atomically:
    /// 1. loads the pending invitation (errors with [`InvitationNotFound`] if
    ///    none exists for the given pair);
    /// 2. verifies it has not expired (`now < expires_at`, errors with
    ///    [`InvitationExpired`] otherwise);
    /// 3. removes it from the pending map;
    /// 4. records `node_id -> family_id` in [`Self::family_members`];
    /// 5. increments the family's `members` counter (errors with
    ///    [`FamilyNotFound`] if the family has somehow been removed);
    /// 6. archives the invitation in [`Self::past_family_invitations`] with
    ///    status [`FamilyInvitationStatus::Accepted`], using the next free
    ///    per-`(family, node)` counter.
    ///
    /// The caller is responsible for verifying that `node_id` is owned by
    /// the transaction sender and is not already a member of any family.
    ///
    /// Returns the updated [`NodeFamily`] (with the bumped `members` count).
    ///
    /// [`InvitationNotFound`]: NodeFamiliesContractError::InvitationNotFound
    /// [`InvitationExpired`]: NodeFamiliesContractError::InvitationExpired
    /// [`FamilyNotFound`]: NodeFamiliesContractError::FamilyNotFound
    pub(crate) fn accept_invitation(
        &self,
        store: &mut dyn Storage,
        env: &Env,
        family_id: NodeFamilyId,
        node_id: NodeId,
    ) -> Result<NodeFamily, NodeFamiliesContractError> {
        let now = env.block.time.seconds();
        let key: FamilyMember = (family_id, node_id);

        let invitation = self
            .pending_family_invitations
            .may_load(store, key)?
            .ok_or(NodeFamiliesContractError::InvitationNotFound { family_id, node_id })?;

        if now >= invitation.expires_at {
            return Err(NodeFamiliesContractError::InvitationExpired {
                family_id,
                node_id,
                expires_at: invitation.expires_at,
                now,
            });
        }

        self.pending_family_invitations.remove(store, key)?;

        self.family_members.save(store, node_id, &family_id)?;

        let mut family = self
            .families
            .may_load(store, family_id)?
            .ok_or(NodeFamiliesContractError::FamilyNotFound { family_id })?;
        family.members += 1;
        self.families.save(store, family_id, &family)?;

        let counter = self.next_past_invitation_counter(store, key)?;
        self.past_family_invitations.save(
            store,
            (key, counter),
            &PastFamilyInvitation {
                invitation,
                status: FamilyInvitationStatus::Accepted { at: now },
            },
        )?;

        Ok(family)
    }
}

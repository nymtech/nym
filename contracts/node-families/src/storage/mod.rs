// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// storage will be used in subsequent PRs/tickets
#![allow(dead_code)]

use crate::storage::storage_indexes::{
    NodeFamiliesIndex, NodeFamilyInvitationIndex, PastFamilyInvitationsIndex,
    PastFamilyMembersIndex,
};
use cosmwasm_std::{Addr, Env, Order, StdResult, Storage};
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

    /// All existing families, keyed by id, with unique secondary indexes on
    /// `owner` (one-family-per-owner-address) and on `name`
    /// (family names are globally unique — compared by raw bytes, so
    /// callers normalise upstream if case-insensitive uniqueness is desired).
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
    /// - normalising `name` (e.g. trim/lowercase) if case-insensitive
    ///   uniqueness is desired — the storage layer compares raw bytes;
    /// - ensuring `owner` does not already own a family **and** is not
    ///   currently a member of one.
    ///
    /// Returns the freshly persisted [`NodeFamily`]. The underlying
    /// `IndexedMap` enforces the one-family-per-owner and unique-name
    /// invariants via unique indexes on `owner` and `name` as a
    /// defence-in-depth check, so this call will fail if either is already
    /// taken — but the caller must not rely on it for the membership check.
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

    /// Persist a new pending invitation for `node_id` to join `family_id`.
    ///
    /// `expires_at` is taken as a unix-seconds absolute deadline (the caller
    /// is expected to compute it from the current block time plus the
    /// configured invitation duration).
    ///
    /// The caller (a transaction handler) is responsible for:
    /// - verifying that `family_id` exists and that the transaction sender
    ///   is its owner;
    /// - verifying that `node_id` refers to a real, registered node;
    /// - ensuring `node_id` is not already a member of any family;
    /// - ensuring `expires_at` is strictly in the future.
    ///
    /// As defence-in-depth, this method errors with [`FamilyNotFound`] if
    /// `family_id` is unknown and with [`PendingInvitationAlreadyExists`] if
    /// a pending invitation for the same `(family, node)` pair is already
    /// stored — the underlying `IndexedMap` would otherwise silently
    /// overwrite it.
    ///
    /// Returns the freshly persisted [`FamilyInvitation`].
    ///
    /// [`FamilyNotFound`]: NodeFamiliesContractError::FamilyNotFound
    /// [`PendingInvitationAlreadyExists`]: NodeFamiliesContractError::PendingInvitationAlreadyExists
    pub(crate) fn add_pending_invitation(
        &self,
        store: &mut dyn Storage,
        family_id: NodeFamilyId,
        node_id: NodeId,
        expires_at: u64,
    ) -> Result<FamilyInvitation, NodeFamiliesContractError> {
        let key: FamilyMember = (family_id, node_id);

        if !self.families.has(store, family_id) {
            return Err(NodeFamiliesContractError::FamilyNotFound { family_id });
        }

        if self
            .pending_family_invitations
            .may_load(store, key)?
            .is_some()
        {
            return Err(NodeFamiliesContractError::PendingInvitationAlreadyExists {
                family_id,
                node_id,
            });
        }

        let invitation = FamilyInvitation {
            family_id,
            node_id,
            expires_at,
        };
        self.pending_family_invitations
            .save(store, key, &invitation)?;
        Ok(invitation)
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

    /// Reject a pending invitation for `node_id` from `family_id`.
    ///
    /// Invitee-side counterpart to [`Self::revoke_pending_invitation`]:
    /// removes the invitation from [`Self::pending_family_invitations`] and
    /// archives it in [`Self::past_family_invitations`] with status
    /// [`FamilyInvitationStatus::Rejected`], using the next free
    /// per-`(family, node)` counter. Errors with [`InvitationNotFound`] if
    /// no pending invitation exists for the given pair.
    ///
    /// Works regardless of whether the invitation has expired.
    ///
    /// The caller is responsible for verifying that the transaction sender
    /// is the controller of `node_id`.
    ///
    /// Returns the rejected [`FamilyInvitation`].
    ///
    /// [`InvitationNotFound`]: NodeFamiliesContractError::InvitationNotFound
    pub(crate) fn reject_pending_invitation(
        &self,
        store: &mut dyn Storage,
        env: &Env,
        family_id: NodeFamilyId,
        node_id: NodeId,
    ) -> Result<FamilyInvitation, NodeFamiliesContractError> {
        let now = env.block.time.seconds();
        let key: FamilyMember = (family_id, node_id);

        let invitation = self
            .pending_family_invitations
            .may_load(store, key)?
            .ok_or(NodeFamiliesContractError::InvitationNotFound { family_id, node_id })?;

        self.pending_family_invitations.remove(store, key)?;

        let counter = self.next_past_invitation_counter(store, key)?;
        self.past_family_invitations.save(
            store,
            (key, counter),
            &PastFamilyInvitation {
                invitation: invitation.clone(),
                status: FamilyInvitationStatus::Rejected { at: now },
            },
        )?;

        Ok(invitation)
    }

    /// Revoke a pending invitation for `node_id` from `family_id`.
    ///
    /// Removes the invitation from [`Self::pending_family_invitations`] and
    /// archives it in [`Self::past_family_invitations`] with status
    /// [`FamilyInvitationStatus::Revoked`], using the next free
    /// per-`(family, node)` counter. Errors with [`InvitationNotFound`] if
    /// no pending invitation exists for the given pair.
    ///
    /// Works regardless of whether the invitation has expired — this is the
    /// only path that can clean expired entries out of the pending map, since
    /// no background sweeper exists.
    ///
    /// The caller is responsible for verifying that the transaction sender
    /// is the owner of `family_id`.
    ///
    /// Returns the revoked [`FamilyInvitation`].
    ///
    /// [`InvitationNotFound`]: NodeFamiliesContractError::InvitationNotFound
    pub(crate) fn revoke_pending_invitation(
        &self,
        store: &mut dyn Storage,
        env: &Env,
        family_id: NodeFamilyId,
        node_id: NodeId,
    ) -> Result<FamilyInvitation, NodeFamiliesContractError> {
        let now = env.block.time.seconds();
        let key: FamilyMember = (family_id, node_id);

        let invitation = self
            .pending_family_invitations
            .may_load(store, key)?
            .ok_or(NodeFamiliesContractError::InvitationNotFound { family_id, node_id })?;

        self.pending_family_invitations.remove(store, key)?;

        let counter = self.next_past_invitation_counter(store, key)?;
        self.past_family_invitations.save(
            store,
            (key, counter),
            &PastFamilyInvitation {
                invitation: invitation.clone(),
                status: FamilyInvitationStatus::Revoked { at: now },
            },
        )?;

        Ok(invitation)
    }

    /// Remove `node_id` from whichever family it currently belongs to.
    ///
    /// Shared storage path for both routes that drop a member:
    /// - **kick** — invoked by the family owner against another node;
    /// - **leave** — invoked by the node's own controller.
    ///
    /// Looks up the node's family via [`Self::family_members`] (errors with
    /// [`NodeNotInFamily`] if the node has no membership record), removes
    /// the membership entry, decrements the family's `members` counter
    /// (saturating at `0` as defence-in-depth — a underflow would indicate
    /// an invariant break elsewhere), and archives a [`PastFamilyMember`]
    /// record stamped with `removed_at = env.block.time.seconds()` using
    /// the next per-`(family, node)` archive slot.
    ///
    /// The caller is responsible for verifying that the transaction sender
    /// is authorised to remove this node — either as the family owner
    /// (kick) or as the node's controller (leave).
    ///
    /// Returns the updated [`NodeFamily`] (with the decremented `members`
    /// count). Errors with [`FamilyNotFound`] if the node's family has
    /// somehow been removed.
    ///
    /// [`NodeNotInFamily`]: NodeFamiliesContractError::NodeNotInFamily
    /// [`FamilyNotFound`]: NodeFamiliesContractError::FamilyNotFound
    pub(crate) fn remove_family_member(
        &self,
        store: &mut dyn Storage,
        env: &Env,
        node_id: NodeId,
    ) -> Result<NodeFamily, NodeFamiliesContractError> {
        let now = env.block.time.seconds();

        let family_id = self
            .family_members
            .may_load(store, node_id)?
            .ok_or(NodeFamiliesContractError::NodeNotInFamily { node_id })?;

        self.family_members.remove(store, node_id);

        let mut family = self
            .families
            .may_load(store, family_id)?
            .ok_or(NodeFamiliesContractError::FamilyNotFound { family_id })?;
        family.members = family.members.saturating_sub(1);
        self.families.save(store, family_id, &family)?;

        let key: FamilyMember = (family_id, node_id);
        let counter = self.next_past_member_counter(store, key)?;
        self.past_family_members.save(
            store,
            (key, counter),
            &PastFamilyMember {
                family_id,
                node_id,
                removed_at: now,
            },
        )?;

        Ok(family)
    }

    /// Disband (delete) `family_id`.
    ///
    /// Only succeeds when the family has **zero current members** — errors
    /// with [`FamilyNotEmpty`] otherwise. The owner is responsible for
    /// kicking any remaining members first.
    ///
    /// Sweeps every still-pending invitation issued by the family
    /// (iterating via the `family` multi-index over
    /// [`Self::pending_family_invitations`]), removing each from the
    /// pending map and archiving it as
    /// [`FamilyInvitationStatus::Revoked`] at `env.block.time` — disbanding
    /// the family is treated as the family withdrawing all of its
    /// outstanding invitations. Gas cost therefore scales with the number
    /// of leftover invitations; if that becomes a concern, the owner can
    /// revoke them manually before disbanding.
    ///
    /// The caller is responsible for verifying that the transaction sender
    /// is the owner of `family_id`.
    ///
    /// Errors with [`FamilyNotFound`] if `family_id` does not exist.
    /// Returns the disbanded [`NodeFamily`] (final snapshot) for use in
    /// event attributes.
    ///
    /// [`FamilyNotEmpty`]: NodeFamiliesContractError::FamilyNotEmpty
    /// [`FamilyNotFound`]: NodeFamiliesContractError::FamilyNotFound
    pub(crate) fn disband_family(
        &self,
        store: &mut dyn Storage,
        env: &Env,
        family_id: NodeFamilyId,
    ) -> Result<NodeFamily, NodeFamiliesContractError> {
        let now = env.block.time.seconds();

        let family = self
            .families
            .may_load(store, family_id)?
            .ok_or(NodeFamiliesContractError::FamilyNotFound { family_id })?;

        if family.members != 0 {
            return Err(NodeFamiliesContractError::FamilyNotEmpty {
                family_id,
                members: family.members,
            });
        }

        // collect first, then mutate — iterating an IndexedMap while modifying it is unsafe
        let pending: Vec<(FamilyMember, FamilyInvitation)> = self
            .pending_family_invitations
            .idx
            .family
            .prefix(family_id)
            .range(store, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;

        for (key, invitation) in pending {
            self.pending_family_invitations.remove(store, key)?;
            let counter = self.next_past_invitation_counter(store, key)?;
            self.past_family_invitations.save(
                store,
                (key, counter),
                &PastFamilyInvitation {
                    invitation,
                    status: FamilyInvitationStatus::Revoked { at: now },
                },
            )?;
        }

        self.families.remove(store, family_id)?;

        Ok(family)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{init_contract_tester, NodeFamiliesContractTesterExt};
    use nym_contracts_common_testing::ContractOpts;

    // ---- counters ----

    #[test]
    fn next_family_id_starts_at_1_and_increments() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();

        assert_eq!(s.next_family_id(tester.storage_mut()).unwrap(), 1);
        assert_eq!(s.next_family_id(tester.storage_mut()).unwrap(), 2);
        assert_eq!(s.next_family_id(tester.storage_mut()).unwrap(), 3);
    }

    #[test]
    fn past_invitation_counter_starts_at_0_per_key() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let k1: FamilyMember = (1, 100);
        let k2: FamilyMember = (2, 100);

        assert_eq!(
            s.next_past_invitation_counter(tester.storage_mut(), k1)
                .unwrap(),
            0
        );
        assert_eq!(
            s.next_past_invitation_counter(tester.storage_mut(), k1)
                .unwrap(),
            1
        );
        // independent counter for a different key
        assert_eq!(
            s.next_past_invitation_counter(tester.storage_mut(), k2)
                .unwrap(),
            0
        );
        assert_eq!(
            s.next_past_invitation_counter(tester.storage_mut(), k1)
                .unwrap(),
            2
        );
    }

    #[test]
    fn past_member_counter_starts_at_0_per_key() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let k: FamilyMember = (1, 100);

        assert_eq!(
            s.next_past_member_counter(tester.storage_mut(), k).unwrap(),
            0
        );
        assert_eq!(
            s.next_past_member_counter(tester.storage_mut(), k).unwrap(),
            1
        );
    }

    // ---- register_new_family ----

    #[test]
    fn register_new_family_persists_with_expected_fields() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();
        let owner = tester.addr_make("alice");

        let family = s
            .register_new_family(
                tester.storage_mut(),
                &env,
                owner.clone(),
                "fam".into(),
                "desc".into(),
            )
            .unwrap();

        assert_eq!(family.id, 1);
        assert_eq!(family.owner, owner);
        assert_eq!(family.name, "fam");
        assert_eq!(family.description, "desc");
        assert_eq!(family.members, 0);
        assert_eq!(family.created_at, tester.env().block.time.seconds());

        let stored = s.families.load(tester.storage(), 1).unwrap();
        assert_eq!(stored, family);
    }

    #[test]
    fn register_new_family_assigns_sequential_ids() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();
        let alice = tester.addr_make("alice");
        let bob = tester.addr_make("bob");

        let f1 = tester.add_dummy_family();
        let f2 = tester.add_dummy_family();

        assert_eq!(f1.id, 1);
        assert_eq!(f2.id, 2);
    }

    #[test]
    fn register_new_family_rejects_duplicate_name() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();
        let alice = tester.addr_make("alice");
        let bob = tester.addr_make("bob");

        s.register_new_family(
            tester.storage_mut(),
            &env,
            alice,
            "shared".into(),
            "".into(),
        )
        .unwrap();

        // unique-index defence-in-depth check
        let res = s.register_new_family(
            tester.storage_mut(),
            &env,
            bob,
            "shared".into(),
            "".into(),
        );
        assert!(res.is_err());
    }

    #[test]
    fn register_new_family_rejects_duplicate_owner() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();
        let alice = tester.addr_make("alice");

        tester.make_family(&alice);

        // unique-index defence-in-depth check
        let res = s.register_new_family(
            tester.storage_mut(),
            &env,
            alice,
            "second".into(),
            "".into(),
        );
        assert!(res.is_err());
    }

    // ---- add_pending_invitation ----

    #[test]
    fn add_pending_invitation_persists() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();
        let alice = tester.addr_make("alice");
        let f = tester.make_family(&alice);
        let expires_at = tester.env().block.time.seconds() + 100;

        let inv = s
            .add_pending_invitation(tester.storage_mut(), f.id, 42, expires_at)
            .unwrap();

        assert_eq!(inv.family_id, f.id);
        assert_eq!(inv.node_id, 42);
        assert_eq!(inv.expires_at, expires_at);
        let stored = s
            .pending_family_invitations
            .load(tester.storage(), (f.id, 42))
            .unwrap();
        assert_eq!(stored, inv);
    }

    #[test]
    fn add_pending_invitation_errors_on_unknown_family() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();
        let expires_at = env.block.time.seconds() + 100;

        let res = s.add_pending_invitation(tester.storage_mut(), 99, 42, expires_at);
        assert_eq!(
            res.unwrap_err(),
            NodeFamiliesContractError::FamilyNotFound { family_id: 99 }
        );
    }

    #[test]
    fn add_pending_invitation_errors_on_duplicate() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();
        let alice = tester.addr_make("alice");
        let f = tester.make_family(&alice);

        tester.invite_to_family(f.id, 42);

        let expires_at = env.block.time.seconds() + 200;
        let res = s.add_pending_invitation(tester.storage_mut(), f.id, 42, expires_at);
        assert_eq!(
            res.unwrap_err(),
            NodeFamiliesContractError::PendingInvitationAlreadyExists {
                family_id: f.id,
                node_id: 42,
            }
        );
    }

    // ---- accept_invitation ----

    #[test]
    fn accept_invitation_happy_path() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();
        let alice = tester.addr_make("alice");
        let f = tester.make_family(&alice);
        let expires_at = env.block.time.seconds() + 100;
        s.add_pending_invitation(tester.storage_mut(), f.id, 42, expires_at)
            .unwrap();

        let updated = s
            .accept_invitation(tester.storage_mut(), &env, f.id, 42)
            .unwrap();

        assert_eq!(updated.members, 1);
        assert!(s
            .pending_family_invitations
            .may_load(tester.storage(), (f.id, 42))
            .unwrap()
            .is_none());
        assert_eq!(s.family_members.load(tester.storage(), 42).unwrap(), f.id);
        assert_eq!(s.families.load(tester.storage(), f.id).unwrap().members, 1);

        let past = s
            .past_family_invitations
            .load(tester.storage(), ((f.id, 42), 0))
            .unwrap();
        assert_eq!(
            past.status,
            FamilyInvitationStatus::Accepted {
                at: tester.env().block.time.seconds()
            }
        );
        assert_eq!(past.invitation.family_id, f.id);
        assert_eq!(past.invitation.node_id, 42);
    }

    #[test]
    fn accept_invitation_errors_when_no_pending() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();

        let res = s.accept_invitation(tester.storage_mut(), &env, 1, 42);
        assert_eq!(
            res.unwrap_err(),
            NodeFamiliesContractError::InvitationNotFound {
                family_id: 1,
                node_id: 42,
            }
        );
    }

    #[test]
    fn accept_invitation_errors_when_expired() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();
        let alice = tester.addr_make("alice");
        let f = tester.make_family(&alice);
        // expires at exactly `now` — `now >= expires_at` triggers
        let expires_at = tester.env().block.time.seconds();
        s.add_pending_invitation(tester.storage_mut(), f.id, 42, expires_at)
            .unwrap();

        let res = s.accept_invitation(tester.storage_mut(), &env, f.id, 42);
        assert_eq!(
            res.unwrap_err(),
            NodeFamiliesContractError::InvitationExpired {
                family_id: f.id,
                node_id: 42,
                expires_at,
                now: tester.env().block.time.seconds(),
            }
        );
    }

    // ---- reject_pending_invitation ----

    #[test]
    fn reject_invitation_happy_path() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();
        let alice = tester.addr_make("alice");
        let f = tester.make_family(&alice);
        tester.invite_to_family(f.id, 42);

        let inv = s
            .reject_pending_invitation(tester.storage_mut(), &env, f.id, 42)
            .unwrap();
        assert_eq!(inv.node_id, 42);
        assert!(s
            .pending_family_invitations
            .may_load(tester.storage(), (f.id, 42))
            .unwrap()
            .is_none());
        let past = s
            .past_family_invitations
            .load(tester.storage(), ((f.id, 42), 0))
            .unwrap();
        assert_eq!(
            past.status,
            FamilyInvitationStatus::Rejected {
                at: tester.env().block.time.seconds()
            }
        );
    }

    #[test]
    fn reject_invitation_works_on_expired() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();
        let alice = tester.addr_make("alice");
        let f = tester.make_family(&alice);
        let expires_at = env.block.time.seconds();
        s.add_pending_invitation(tester.storage_mut(), f.id, 42, expires_at)
            .unwrap();

        s.reject_pending_invitation(tester.storage_mut(), &env, f.id, 42)
            .unwrap();
    }

    #[test]
    fn reject_invitation_errors_when_no_pending() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();

        let res = s.reject_pending_invitation(tester.storage_mut(), &env, 1, 42);
        assert_eq!(
            res.unwrap_err(),
            NodeFamiliesContractError::InvitationNotFound {
                family_id: 1,
                node_id: 42,
            }
        );
    }

    // ---- revoke_pending_invitation ----

    #[test]
    fn revoke_invitation_happy_path() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();
        let alice = tester.addr_make("alice");
        let f = tester.make_family(&alice);
        tester.invite_to_family(f.id, 42);

        s.revoke_pending_invitation(tester.storage_mut(), &env, f.id, 42)
            .unwrap();
        let past = s
            .past_family_invitations
            .load(tester.storage(), ((f.id, 42), 0))
            .unwrap();
        assert_eq!(
            past.status,
            FamilyInvitationStatus::Revoked {
                at: tester.env().block.time.seconds()
            }
        );
    }

    #[test]
    fn revoke_invitation_errors_when_no_pending() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();

        let res = s.revoke_pending_invitation(tester.storage_mut(), &env, 1, 42);
        assert_eq!(
            res.unwrap_err(),
            NodeFamiliesContractError::InvitationNotFound {
                family_id: 1,
                node_id: 42,
            }
        );
    }

    // ---- remove_family_member ----

    #[test]
    fn remove_family_member_happy_path() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();
        let alice = tester.addr_make("alice");
        let f = tester.make_family(&alice);
        tester.add_to_family(f.id, 42);

        let updated = s
            .remove_family_member(tester.storage_mut(), &env, 42)
            .unwrap();

        assert_eq!(updated.members, 0);
        assert!(s
            .family_members
            .may_load(tester.storage(), 42)
            .unwrap()
            .is_none());
        let past = s
            .past_family_members
            .load(tester.storage(), ((f.id, 42), 0))
            .unwrap();
        assert_eq!(past.family_id, f.id);
        assert_eq!(past.node_id, 42);
        assert_eq!(past.removed_at, tester.env().block.time.seconds());
    }

    #[test]
    fn remove_family_member_errors_when_node_not_in_any_family() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();

        let res = s.remove_family_member(tester.storage_mut(), &env, 999);
        assert_eq!(
            res.unwrap_err(),
            NodeFamiliesContractError::NodeNotInFamily { node_id: 999 }
        );
    }

    #[test]
    fn remove_family_member_uses_per_pair_archive_counter() {
        // joining and leaving the same family twice must not collide on the archive key
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();
        let alice = tester.addr_make("alice");
        let f = tester.make_family(&alice);

        let expires_at = env.block.time.seconds() + 100;
        for _ in 0..2 {
            s.add_pending_invitation(tester.storage_mut(), f.id, 42, expires_at)
                .unwrap();
            s.accept_invitation(tester.storage_mut(), &env, f.id, 42)
                .unwrap();
            s.remove_family_member(tester.storage_mut(), &env, 42)
                .unwrap();
        }

        // both archive slots present
        s.past_family_members
            .load(tester.storage(), ((f.id, 42), 0))
            .unwrap();
        s.past_family_members
            .load(tester.storage(), ((f.id, 42), 1))
            .unwrap();
    }

    // ---- disband_family ----

    #[test]
    fn disband_family_happy_path_no_pending() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();
        let alice = tester.addr_make("alice");
        let f = tester.make_family(&alice);

        let snap = s.disband_family(tester.storage_mut(), &env, f.id).unwrap();
        assert_eq!(snap.id, f.id);
        assert!(s
            .families
            .may_load(tester.storage(), f.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn disband_family_sweeps_all_pending_invitations_as_revoked() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();
        let alice = tester.addr_make("alice");
        let f = tester.make_family(&alice);
        tester.invite_to_family(f.id, 42);
        tester.invite_to_family(f.id, 43);

        s.disband_family(tester.storage_mut(), &env, f.id).unwrap();

        assert!(s
            .pending_family_invitations
            .may_load(tester.storage(), (f.id, 42))
            .unwrap()
            .is_none());
        assert!(s
            .pending_family_invitations
            .may_load(tester.storage(), (f.id, 43))
            .unwrap()
            .is_none());
        assert_eq!(
            s.past_family_invitations
                .load(tester.storage(), ((f.id, 42), 0))
                .unwrap()
                .status,
            FamilyInvitationStatus::Revoked {
                at: tester.env().block.time.seconds()
            }
        );
        assert_eq!(
            s.past_family_invitations
                .load(tester.storage(), ((f.id, 43), 0))
                .unwrap()
                .status,
            FamilyInvitationStatus::Revoked {
                at: tester.env().block.time.seconds()
            }
        );
    }

    #[test]
    fn disband_family_errors_when_not_empty() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();
        let alice = tester.addr_make("alice");
        let f = tester.make_family(&alice);
        tester.add_to_family(f.id, 42);

        let res = s.disband_family(tester.storage_mut(), &env, f.id);
        assert_eq!(
            res.unwrap_err(),
            NodeFamiliesContractError::FamilyNotEmpty {
                family_id: f.id,
                members: 1,
            }
        );
    }

    #[test]
    fn disband_family_errors_on_unknown_family() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();

        let res = s.disband_family(tester.storage_mut(), &env, 99);
        assert_eq!(
            res.unwrap_err(),
            NodeFamiliesContractError::FamilyNotFound { family_id: 99 }
        );
    }

    #[test]
    fn after_disband_owner_can_register_again_with_new_id() {
        let mut tester = init_contract_tester();
        let s = NodeFamiliesStorage::new();
        let env = tester.env();
        let alice = tester.addr_make("alice");

        let f1 = tester.make_family(&alice);
        s.disband_family(tester.storage_mut(), &env, f1.id).unwrap();
        let f2 = s
            .register_new_family(tester.storage_mut(), &env, alice, "2".into(), "".into())
            .unwrap();

        // ids monotonically increase, never recycled
        assert_eq!(f1.id, 1);
        assert_eq!(f2.id, 2);
    }
}

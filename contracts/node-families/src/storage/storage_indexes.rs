// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::storage::FamilyMember;
use cosmwasm_std::Addr;
use cw_storage_plus::{Index, IndexList, MultiIndex, UniqueIndex};
use node_families_contract_common::constants::storage_keys;
use node_families_contract_common::{
    FamilyInvitation, NodeFamily, NodeFamilyId, PastFamilyInvitation, PastFamilyMember,
};
use nym_mixnet_contract_common::NodeId;

/// Secondary indexes over [`NodeFamily`]. Enforces one-family-per-owner via
/// a `UniqueIndex` on the owner address.
pub(crate) struct NodeFamiliesIndex<'a> {
    /// Unique index: at most one family per owner [`Addr`].
    pub(crate) owner: UniqueIndex<'a, Addr, NodeFamily, NodeFamilyId>,
}

impl<'a> NodeFamiliesIndex<'a> {
    #[allow(clippy::new_without_default)]
    pub(crate) fn new() -> Self {
        NodeFamiliesIndex {
            owner: UniqueIndex::new(
                |family| family.owner.clone(),
                storage_keys::FAMILIES_OWNER_IDX_NAMESPACE,
            ),
        }
    }
}

impl IndexList<NodeFamily> for NodeFamiliesIndex<'_> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<NodeFamily>> + '_> {
        Box::new(std::iter::once(&self.owner as &dyn Index<NodeFamily>))
    }
}

/// Secondary indexes over pending [`FamilyInvitation`]s, allowing lookup by
/// either family id or node id.
pub(crate) struct NodeFamilyInvitationIndex<'a> {
    /// Multi-index: all pending invitations issued by a given family.
    pub(crate) family: MultiIndex<'a, NodeFamilyId, FamilyInvitation, FamilyMember>,
    /// Multi-index: all pending invitations addressed to a given node.
    pub(crate) node: MultiIndex<'a, NodeId, FamilyInvitation, FamilyMember>,
}

impl<'a> NodeFamilyInvitationIndex<'a> {
    pub(crate) fn new() -> Self {
        NodeFamilyInvitationIndex {
            family: MultiIndex::new(
                |_pk, inv| inv.family_id,
                storage_keys::INVITATIONS_NAMESPACE,
                storage_keys::INVITATIONS_FAMILY_IDX_NAMESPACE,
            ),
            node: MultiIndex::new(
                |_pk, inv| inv.node_id,
                storage_keys::INVITATIONS_NAMESPACE,
                storage_keys::INVITATIONS_NODE_IDX_NAMESPACE,
            ),
        }
    }
}

impl IndexList<FamilyInvitation> for NodeFamilyInvitationIndex<'_> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<FamilyInvitation>> + '_> {
        let v: Vec<&dyn Index<FamilyInvitation>> = vec![&self.family, &self.node];
        Box::new(v.into_iter())
    }
}

/// Secondary indexes over the [`PastFamilyMember`] archive.
pub(crate) struct PastFamilyMembersIndex<'a> {
    /// Multi-index: every past membership record for a given family.
    pub(crate) family: MultiIndex<'a, NodeFamilyId, PastFamilyMember, (FamilyMember, u64)>,
    /// Multi-index: every past membership record for a given node.
    pub(crate) node: MultiIndex<'a, NodeId, PastFamilyMember, (FamilyMember, u64)>,
}

impl<'a> PastFamilyMembersIndex<'a> {
    #[allow(clippy::new_without_default)]
    pub(crate) fn new() -> Self {
        PastFamilyMembersIndex {
            family: MultiIndex::new(
                |_pk, mem| mem.family_id,
                storage_keys::PAST_FAMILY_MEMBER_NAMESPACE,
                storage_keys::PAST_FAMILY_MEMBER_FAMILY_IDX_NAMESPACE,
            ),
            node: MultiIndex::new(
                |_pk, mem| mem.node_id,
                storage_keys::PAST_FAMILY_MEMBER_NAMESPACE,
                storage_keys::PAST_FAMILY_MEMBER_NODE_IDX_NAMESPACE,
            ),
        }
    }
}

impl IndexList<PastFamilyMember> for PastFamilyMembersIndex<'_> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<PastFamilyMember>> + '_> {
        let v: Vec<&dyn Index<PastFamilyMember>> = vec![&self.family, &self.node];
        Box::new(v.into_iter())
    }
}

/// Secondary indexes over the [`PastFamilyInvitation`] archive
/// (rejected / revoked invitations).
pub(crate) struct PastFamilyInvitationsIndex<'a> {
    /// Multi-index: every archived invitation issued by a given family.
    pub(crate) family: MultiIndex<'a, NodeFamilyId, PastFamilyInvitation, (FamilyMember, u64)>,
    /// Multi-index: every archived invitation addressed to a given node.
    pub(crate) node: MultiIndex<'a, NodeId, PastFamilyInvitation, (FamilyMember, u64)>,
}

impl<'a> PastFamilyInvitationsIndex<'a> {
    #[allow(clippy::new_without_default)]
    pub(crate) fn new() -> Self {
        PastFamilyInvitationsIndex {
            family: MultiIndex::new(
                |_pk, inv| inv.invitation.family_id,
                storage_keys::PAST_INVITATIONS_NAMESPACE,
                storage_keys::PAST_INVITATIONS_FAMILY_IDX_NAMESPACE,
            ),
            node: MultiIndex::new(
                |_pk, inv| inv.invitation.node_id,
                storage_keys::PAST_INVITATIONS_NAMESPACE,
                storage_keys::PAST_INVITATIONS_NODE_IDX_NAMESPACE,
            ),
        }
    }
}

impl IndexList<PastFamilyInvitation> for PastFamilyInvitationsIndex<'_> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<PastFamilyInvitation>> + '_> {
        let v: Vec<&dyn Index<PastFamilyInvitation>> = vec![&self.family, &self.node];
        Box::new(v.into_iter())
    }
}

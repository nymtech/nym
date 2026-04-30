// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use nym_mixnet_contract_common::NodeId;

/// Identifier of a node family.
///
/// Issued sequentially by the contract on family creation; never reused even if the
/// family is later disbanded.
pub type NodeFamilyId = u32;

/// Runtime configuration of the node families contract.
pub struct Config {
    /// Fee charged on each successful `create_family` execution.
    pub create_family_fee: Coin,

    /// Maximum allowed length, in characters, of a family name.
    pub family_name_length_limit: usize,

    /// Maximum allowed length, in characters, of a family description.
    pub family_description_length_limit: usize,
}

/// On-chain representation of a node family.
#[cw_serde]
pub struct NodeFamily {
    /// The id of the node family
    pub id: NodeFamilyId,

    /// The name of the node family
    pub name: String,

    /// The optional description of the node family
    pub description: String,

    /// The owner of the node family
    pub owner: Addr,

    /// Memoized value of the current number of members in the node family
    /// Used to detect if the family is empty
    pub members: u64,

    /// Timestamp of the creation of the node family
    pub created_at: u64,
}

/// A pending invitation for a node to join a particular family.
///
/// Invitations are stored until they are accepted, rejected, revoked, or until the
/// chain advances past `expires_at` (in which case they remain in storage but are
/// treated as inert — there is no background process clearing expired invitations).
#[cw_serde]
pub struct FamilyInvitation {
    /// The family that issued the invitation.
    pub family_id: NodeFamilyId,

    /// The node being invited.
    pub node_id: NodeId,

    /// Block timestamp (unix seconds) after which the invitation is no longer valid.
    pub expires_at: u64,
}

/// On-chain record of a node's current family membership.
///
/// A node belongs to at most one family at a time, so this is keyed by
/// `NodeId` alone — `family_id` is carried in the value to support reverse
/// lookups (all nodes in a given family) via a secondary index.
#[cw_serde]
pub struct FamilyMembership {
    /// The family the node is currently a member of.
    pub family_id: NodeFamilyId,

    /// Block timestamp (unix seconds) at which the node accepted its
    /// invitation and joined the family.
    pub joined_at: u64,
}

/// Historical record of a node that used to be part of a family but has since been
/// removed (kicked, left voluntarily, or because the family was disbanded).
#[cw_serde]
pub struct PastFamilyMember {
    /// The family the node used to belong to.
    pub family_id: NodeFamilyId,

    /// The node that was removed.
    pub node_id: NodeId,

    /// Block timestamp (unix seconds) at which the membership was terminated.
    pub removed_at: u64,
}

/// Terminal status for an invitation that has been moved out of the pending set.
///
/// Note: timed-out invitations are not represented here — they are simply left in
/// the pending set (see `FamilyInvitation::expires_at`).
#[cw_serde]
pub enum FamilyInvitationStatus {
    /// Still awaiting a response. Recorded with a timestamp for completeness even
    /// though pending invitations live in a separate map.
    Pending { at: u64 },
    /// The invitee accepted and joined the family at the given timestamp.
    Accepted { at: u64 },
    /// The invitee explicitly rejected the invitation at the given timestamp.
    Rejected { at: u64 },
    /// The family revoked the invitation at the given timestamp before it could
    /// be accepted or rejected.
    Revoked { at: u64 },
}

/// Historical record of an invitation that has reached a terminal state
/// (`Accepted`, `Rejected`, or `Revoked`). Timed-out invitations are **not**
/// archived here — they remain in the pending map until explicitly cleared.
#[cw_serde]
pub struct PastFamilyInvitation {
    /// The original invitation as it was issued.
    pub invitation: FamilyInvitation,

    /// What ultimately happened to it.
    pub status: FamilyInvitationStatus,
}

/// Response to [`QueryMsg::GetFamilyById`](crate::QueryMsg::GetFamilyById).
#[cw_serde]
pub struct NodeFamilyResponse {
    /// The id that was queried, echoed back so paginated callers can correlate.
    pub family_id: NodeFamilyId,

    /// The matching family, or `None` if no family with `family_id` exists.
    pub family: Option<NodeFamily>,
}

/// Response to [`QueryMsg::GetFamilyMembership`](crate::QueryMsg::GetFamilyMembership).
#[cw_serde]
pub struct NodeFamilyMembershipResponse {
    /// The node that was queried.
    pub node_id: NodeId,

    /// The id of the family the node currently belongs to, or `None` if the
    /// node is not currently a member of any family.
    pub family_id: Option<NodeFamilyId>,
}

/// A pending [`FamilyInvitation`] paired with whether it has already timed
/// out at the time the query was served.
#[cw_serde]
pub struct PendingFamilyInvitationDetails {
    /// The stored invitation as it was issued.
    pub invitation: FamilyInvitation,

    /// `true` iff `now >= invitation.expires_at` at query time, i.e. the
    /// invitation is still in the pending map but can no longer be acted on.
    pub expired: bool,
}

/// Response to [`QueryMsg::GetPendingInvitation`](crate::QueryMsg::GetPendingInvitation).
#[cw_serde]
pub struct PendingFamilyInvitationResponse {
    /// The family component of the queried `(family_id, node_id)` key.
    pub family_id: NodeFamilyId,

    /// The node component of the queried `(family_id, node_id)` key.
    pub node_id: NodeId,

    /// The matching pending invitation along with an explicit expiry flag,
    /// or `None` if no such invitation exists.
    pub invitation: Option<PendingFamilyInvitationDetails>,
}

/// One entry in a [`FamilyMembersPagedResponse`] page — pairs a node id with
/// its [`FamilyMembership`] record (notably its `joined_at` timestamp).
#[cw_serde]
pub struct FamilyMemberRecord {
    /// The node currently in the family.
    pub node_id: NodeId,

    /// The membership record (carries `family_id` and `joined_at`).
    pub membership: FamilyMembership,
}

/// Response to [`QueryMsg::GetFamilyMembersPaged`](crate::QueryMsg::GetFamilyMembersPaged).
#[cw_serde]
pub struct FamilyMembersPagedResponse {
    /// The family whose members were queried, echoed back so paginated
    /// callers can correlate.
    pub family_id: NodeFamilyId,

    /// The members on this page, in ascending [`NodeId`] order.
    pub members: Vec<FamilyMemberRecord>,

    /// Cursor to pass as `start_after` on the next call, or `None` if this
    /// page is empty (which the caller should treat as end-of-list).
    pub start_next_after: Option<NodeId>,
}

/// Response to [`QueryMsg::GetPendingInvitationsForFamilyPaged`](crate::QueryMsg::GetPendingInvitationsForFamilyPaged).
#[cw_serde]
pub struct PendingFamilyInvitationsPagedResponse {
    /// The family whose pending invitations were queried, echoed back so
    /// paginated callers can correlate.
    pub family_id: NodeFamilyId,

    /// The pending invitations on this page, in ascending invitee
    /// [`NodeId`] order, each stamped with whether it had already timed out
    /// at the time the query was served.
    pub invitations: Vec<PendingFamilyInvitationDetails>,

    /// Cursor (last invitee node id) to pass as `start_after` on the next
    /// call, or `None` if this page is empty (treat as end-of-list).
    pub start_next_after: Option<NodeId>,
}

/// Response to [`QueryMsg::GetPendingInvitationsForNodePaged`](crate::QueryMsg::GetPendingInvitationsForNodePaged).
#[cw_serde]
pub struct PendingInvitationsForNodePagedResponse {
    /// The node whose pending invitations were queried, echoed back so
    /// paginated callers can correlate.
    pub node_id: NodeId,

    /// The pending invitations addressed to this node on this page, in
    /// ascending [`NodeFamilyId`] order, each stamped with whether it had
    /// already timed out at the time the query was served.
    pub invitations: Vec<PendingFamilyInvitationDetails>,

    /// Cursor (last issuing family id) to pass as `start_after` on the
    /// next call, or `None` if this page is empty (treat as end-of-list).
    pub start_next_after: Option<NodeFamilyId>,
}

/// Response to [`QueryMsg::GetAllPendingInvitationsPaged`](crate::QueryMsg::GetAllPendingInvitationsPaged).
#[cw_serde]
pub struct PendingInvitationsPagedResponse {
    /// The pending invitations on this page, in ascending
    /// `(family_id, node_id)` order, each stamped with whether it had
    /// already timed out at the time the query was served.
    pub invitations: Vec<PendingFamilyInvitationDetails>,

    /// Cursor (last `(family_id, node_id)` pair) to pass as `start_after`
    /// on the next call, or `None` if this page is empty (treat as
    /// end-of-list).
    pub start_next_after: Option<(NodeFamilyId, NodeId)>,
}

/// Cursor for paginating per-family past-invitation listings: identifies a
/// single archive entry within a family by `(node_id, counter)`. The
/// `counter` is the per-`(family, node)` archive slot — multiple archived
/// invitations can exist for the same `(family, node)` pair (a node may be
/// invited and have the invitation reach a terminal state more than once).
pub type PastFamilyInvitationCursor = (NodeId, u64);

/// Cursor for paginating per-node past-invitation listings: identifies a
/// single archive entry addressed to a fixed node by `(family_id, counter)`.
pub type PastFamilyInvitationForNodeCursor = (NodeFamilyId, u64);

/// Cursor for paginating global past-invitation listings: identifies a
/// single archive entry across all families by `((family_id, node_id), counter)`.
pub type GlobalPastFamilyInvitationCursor = ((NodeFamilyId, NodeId), u64);

/// Response to [`QueryMsg::GetPastInvitationsForFamilyPaged`](crate::QueryMsg::GetPastInvitationsForFamilyPaged).
#[cw_serde]
pub struct PastFamilyInvitationsPagedResponse {
    /// The family whose archived invitations were queried, echoed back so
    /// paginated callers can correlate.
    pub family_id: NodeFamilyId,

    /// The archived invitations on this page, in ascending
    /// `(node_id, counter)` order across all terminal statuses.
    pub invitations: Vec<PastFamilyInvitation>,

    /// Cursor to pass as `start_after` on the next call, or `None` if this
    /// page is empty (treat as end-of-list).
    pub start_next_after: Option<PastFamilyInvitationCursor>,
}

/// Response to [`QueryMsg::GetPastInvitationsForNodePaged`](crate::QueryMsg::GetPastInvitationsForNodePaged).
#[cw_serde]
pub struct PastFamilyInvitationsForNodePagedResponse {
    /// The node whose past invitations were queried, echoed back so
    /// paginated callers can correlate.
    pub node_id: NodeId,

    /// The archived invitations addressed to this node on this page, in
    /// ascending `(family_id, counter)` order across all terminal statuses.
    pub invitations: Vec<PastFamilyInvitation>,

    /// Cursor to pass as `start_after` on the next call, or `None` if this
    /// page is empty (treat as end-of-list).
    pub start_next_after: Option<PastFamilyInvitationForNodeCursor>,
}

/// Response to [`QueryMsg::GetAllPastInvitationsPaged`](crate::QueryMsg::GetAllPastInvitationsPaged).
#[cw_serde]
pub struct AllPastFamilyInvitationsPagedResponse {
    /// The archived invitations on this page, in ascending
    /// `((family_id, node_id), counter)` order across all terminal statuses.
    pub invitations: Vec<PastFamilyInvitation>,

    /// Cursor to pass as `start_after` on the next call, or `None` if this
    /// page is empty (treat as end-of-list).
    pub start_next_after: Option<GlobalPastFamilyInvitationCursor>,
}

/// Response to [`QueryMsg::GetFamiliesPaged`](crate::QueryMsg::GetFamiliesPaged).
#[cw_serde]
pub struct FamiliesPagedResponse {
    /// The families on this page, in ascending [`NodeFamilyId`] order.
    pub families: Vec<NodeFamily>,

    /// Cursor to pass as `start_after` on the next call, or `None` if this
    /// page is empty (which the caller should treat as end-of-list).
    pub start_next_after: Option<NodeFamilyId>,
}

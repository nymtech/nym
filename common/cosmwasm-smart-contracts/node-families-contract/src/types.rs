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

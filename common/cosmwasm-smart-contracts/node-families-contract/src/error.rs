// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::NodeFamilyId;
use cosmwasm_std::{Addr, Coin};
use cw_controllers::AdminError;
use cw_utils::PaymentError;
use nym_mixnet_contract_common::NodeId;
use thiserror::Error;

/// Errors returned from any entry point of the node families contract.
#[derive(Error, Debug, PartialEq)]
pub enum NodeFamiliesContractError {
    /// Returned from `migrate` when the on-chain state cannot be brought forward
    /// to the current contract version (e.g. unsupported source version, malformed
    /// stored data).
    #[error("could not perform contract migration: {comment}")]
    FailedMigration { comment: String },

    /// The referenced family does not exist (or no longer exists).
    #[error("family with id {family_id} does not exist")]
    FamilyNotFound { family_id: NodeFamilyId },

    /// Disbanding was requested on a family that still has members.
    #[error("family {family_id} cannot be disbanded: it still has {members} member(s)")]
    FamilyNotEmpty {
        family_id: NodeFamilyId,
        members: u64,
    },

    /// The given node is not currently a member of any family.
    #[error("node {node_id} is not currently a member of any family")]
    NodeNotInFamily { node_id: NodeId },

    /// The given node is a member of a different family than the one the
    /// caller is acting on. Distinct from [`NodeNotInFamily`] (which means the
    /// node has no membership at all) — surfaces when, e.g., a family owner
    /// tries to kick a node that belongs to someone else's family.
    #[error("node {node_id} is not a member of family {family_id}")]
    NodeNotMemberOfFamily {
        node_id: NodeId,
        family_id: NodeFamilyId,
    },

    /// No pending invitation exists for the given `(family, node)` pair.
    #[error("no pending invitation for node {node_id} from family {family_id}")]
    InvitationNotFound {
        family_id: NodeFamilyId,
        node_id: NodeId,
    },

    /// A pending invitation for the given `(family, node)` pair already exists;
    /// issuing a new one would silently overwrite it.
    #[error("a pending invitation for node {node_id} from family {family_id} already exists")]
    PendingInvitationAlreadyExists {
        family_id: NodeFamilyId,
        node_id: NodeId,
    },

    /// The invitation exists but its `expires_at` is at or before the current
    /// block time, so it can no longer be acted on.
    #[error(
        "invitation for node {node_id} from family {family_id} expired at {expires_at} (now: {now})"
    )]
    InvitationExpired {
        family_id: NodeFamilyId,
        node_id: NodeId,
        expires_at: u64,
        now: u64,
    },

    // AI-DEV: add comments here
    #[error("invalid fee provided: {0}")]
    InvalidDeposit(#[from] PaymentError),

    /// The funds attached to a `CreateFamily` execution don't match the
    /// configured `create_family_fee`.
    #[error("expected exactly {expected} as family creation fee; received {received:?}")]
    InvalidFamilyCreationFee { expected: Coin, received: Vec<Coin> },

    /// The submitted family name normalised to the empty string (i.e. it
    /// contained no ASCII alphanumeric characters).
    #[error("family name cannot be empty after normalisation")]
    EmptyFamilyName,

    /// The submitted family name exceeds the configured length limit.
    #[error("family name length {length} exceeds the configured limit of {limit}")]
    FamilyNameTooLong { length: usize, limit: usize },

    /// The submitted family description exceeds the configured length limit.
    #[error("family description length {length} exceeds the configured limit of {limit}")]
    FamilyDescriptionTooLong { length: usize, limit: usize },

    /// The transaction sender already owns a family.
    #[error("address {address} already owns family {family_id}")]
    SenderAlreadyOwnsAFamily {
        address: Addr,
        family_id: NodeFamilyId,
    },

    /// The transaction sender does not currently own any family - emitted by
    /// owner-gated operations like `disband_family` when the sender has
    /// nothing to act on.
    #[error("address {address} does not currently own any family")]
    SenderDoesntOwnAFamily { address: Addr },

    /// The transaction sender is not the controller of the bonded node
    /// referenced by the message. Covers all of: sender controls no bonded
    /// node, sender controls a different node id, and sender's node has
    /// entered the unbonding state.
    #[error("address {address} is not the controller of bonded node {node_id}")]
    SenderDoesntControlNode { address: Addr, node_id: NodeId },

    /// A family with the requested (normalised) name already exists.
    #[error("a family with name {name:?} already exists (id {family_id})")]
    FamilyNameAlreadyTaken {
        name: String,
        family_id: NodeFamilyId,
    },

    /// A node controlled by the address is currently a member of a family,
    /// so the address cannot also become a family owner or join another family.
    #[error("address {address} controls node {node_id} which is currently in family {family_id}")]
    AlreadyInFamily {
        address: Addr,
        node_id: NodeId,
        family_id: NodeFamilyId,
    },

    /// The node referenced by an invitation does not exist as a bonded node
    /// in the mixnet contract (or has already unbonded).
    #[error("node {node_id} is not a bonded node in the mixnet contract")]
    NodeDoesntExist { node_id: NodeId },

    /// The node referenced by an invitation is already a member of a family,
    /// so it cannot be invited to another one until it leaves / is removed.
    #[error("node {node_id} is already a member of family {family_id}")]
    NodeAlreadyInFamily {
        node_id: NodeId,
        family_id: NodeFamilyId,
    },

    /// The sender supplied a `validity_secs` of `0` for an invitation, which
    /// would create one that is already expired at the moment it is stored.
    #[error("invitation validity must be strictly positive")]
    ZeroInvitationValidity,

    /// Wraps errors raised by `cw-controllers::Admin` (e.g. caller is not admin).
    #[error(transparent)]
    Admin(#[from] AdminError),

    /// Wraps any underlying `cosmwasm_std::StdError` (storage, serialization, etc.).
    #[error(transparent)]
    StdErr(#[from] cosmwasm_std::StdError),
}

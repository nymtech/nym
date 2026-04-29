// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::NodeFamilyId;
use cw_controllers::AdminError;
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

    /// Wraps errors raised by `cw-controllers::Admin` (e.g. caller is not admin).
    #[error(transparent)]
    Admin(#[from] AdminError),

    /// Wraps any underlying `cosmwasm_std::StdError` (storage, serialization, etc.).
    #[error(transparent)]
    StdErr(#[from] cosmwasm_std::StdError),
}

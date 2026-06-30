// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Addr;
use cw_controllers::AdminError;
use nym_mixnet_contract_common::NodeId;
use thiserror::Error;

/// Errors returned from any entry point of the directory contract.
#[derive(Error, Debug, PartialEq)]
pub enum DirectoryContractError {
    /// The supplied ed25519 signature did not verify against the node's identity key.
    #[error("the provided ed25519 signature did not verify")]
    InvalidSignature,

    /// The signed sequence did not equal the node's expected next sequence
    /// (covers replays, replay-after-delete, and skipped / jumped-ahead values).
    #[error("expected sequence {expected} for node {node_id}, got {provided}")]
    InvalidSequence {
        node_id: NodeId,
        expected: u64,
        provided: u64,
    },

    /// The referenced node is not a bonded node in the mixnet contract.
    #[error("node {node_id} is not a bonded node in the mixnet contract")]
    NodeNotBonded { node_id: NodeId },

    /// The node's identity key could not be recovered/decoded from its mixnet bond.
    #[error(
        "could not recover a valid ed25519 identity key for node {node_id} from its mixnet bond"
    )]
    InvalidIdentityKey { node_id: NodeId },

    /// A write referenced a label that is not in the admin-managed whitelist.
    #[error("label {label:?} is not in the allowed set")]
    LabelNotAllowed { label: String },

    /// The `data` length exceeded the label's configured `max_size`.
    #[error("data for label {label:?} is {len} bytes, exceeding its {max} byte limit")]
    DataTooLarge { label: String, len: usize, max: u32 },

    /// An admin tried to set a label `max_size` above the contract ceiling.
    #[error("requested max_size {requested} exceeds the contract ceiling of {ceiling} bytes")]
    MaxSizeAboveCeiling { requested: u32, ceiling: u32 },

    /// A cross-contract callback was received from a sender other than the
    /// configured mixnet contract.
    #[error("address {sender} is not authorised to invoke the mixnet-contract callback")]
    UnauthorisedMixnetCallback { sender: Addr },

    /// Attempted to deserialise an invalid namespace tag
    #[error("{0} is not a valid namespace tag")]
    InvalidNamespace(u8),

    /// A raw entry storage key could not be parsed back into an `EntryKey`
    /// (empty, truncated, a wrong-width node id, or a non-UTF-8 label/suffix).
    #[error("malformed entry storage key: {0}")]
    MalformedStorageKey(String),

    /// A stored entry value could not be decoded by its `try_from_bytes` codec
    /// (truncated, or a length prefix that overruns the buffer).
    #[error("malformed entry value: {0}")]
    MalformedEntryValue(String),

    /// `migrate` could not bring the on-chain state forward.
    #[error("could not perform contract migration: {comment}")]
    FailedMigration { comment: String },

    /// Wraps `cw-controllers::Admin` errors (e.g. caller is not the admin).
    #[error(transparent)]
    Admin(#[from] AdminError),

    /// Wraps any underlying `cosmwasm_std::StdError`.
    #[error(transparent)]
    StdErr(#[from] cosmwasm_std::StdError),
}

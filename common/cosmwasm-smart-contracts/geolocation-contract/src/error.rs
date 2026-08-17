// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::SubjectClass;
use cosmwasm_std::Addr;
use cw_controllers::AdminError;
use nym_mixnet_contract_common::NodeId;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum GeolocationContractError {
    /// A subject id whose byte width did not match the fixed width its class requires.
    /// The width has to be constant within a class, otherwise the storage key's length
    /// prefix varies and entries stop ordering by id content.
    #[error("subject id for class {class} must be {expected} bytes, got {actual}")]
    InvalidSubjectId {
        class: SubjectClass,
        expected: usize,
        actual: usize,
    },

    /// The payload's `content` exceeded the configured maximum size.
    #[error("payload content is {len} bytes, exceeding the {max} byte limit")]
    PayloadTooLarge { len: usize, max: u32 },

    /// A payload was decoded against a version it was not written under. The contract never
    /// raises this, since it stores payloads opaquely; it is for producers and consumers.
    #[error("expected a version {expected} payload, got version {got}")]
    UnexpectedPayloadVersion { expected: u8, got: u8 },

    /// A payload's `content` did not decode under its own version's format.
    #[error("malformed payload content: {0}")]
    MalformedPayload(String),

    /// The sender is not on the agent whitelist at all.
    #[error("address {agent} is not a whitelisted agent")]
    NotWhitelisted { agent: Addr },

    /// The sender is whitelisted but lacks the flag this particular write needs. Kept
    /// distinct from [`Self::NotWhitelisted`] because the two say different things to an
    /// operator: one is "you were never authorised", the other "you were, but not for this".
    #[error("agent {agent} lacks the {permission} permission")]
    MissingAgentPermission {
        agent: Addr,
        permission: &'static str,
    },

    /// A batch carried more entries than the configured maximum.
    #[error("batch holds {size} entries, exceeding the {max} entry limit")]
    BatchTooLarge { size: usize, max: u32 },

    /// The same subject appeared twice in one relay batch.
    ///
    /// Measurements deliberately allow a repeated key, resolving to the last write, but a
    /// self-declaration cannot: monotonicity is checked against stored state, so two
    /// declarations for one node in a batch would both pass and the last-written one could be
    /// the older. Rejecting the batch keeps validity independent of the order it arrives in,
    /// which resolving the duplicate would not.
    #[error("node {node_id} appears more than once in the same relay batch")]
    DuplicateDeclaration { node_id: NodeId },

    /// A node's stored identity key could not be decoded into 32 raw ed25519 bytes.
    #[error("node {node_id} has a malformed identity key in its mixnet bond")]
    InvalidIdentityKey { node_id: NodeId },

    /// A self-declaration's signature did not verify against the node's identity key.
    #[error("the self-declaration's signature did not verify against the node's identity key")]
    InvalidSignature,

    /// A self-declaration that does not supersede the one already stored: a replay of a
    /// superseded artifact, or of the current one.
    #[error("node {node_id} declared at {declared_at}, not after the stored {stored}")]
    StaleDeclaration {
        node_id: NodeId,
        declared_at: u64,
        stored: u64,
    },

    /// A self-declaration stamped further ahead of block time than the skew allows. Kept
    /// distinct from [`Self::StaleDeclaration`] because it is an operational fault - a node
    /// clock running fast - rather than a replay, and presents as "the geolocator is broken"
    /// if the two are conflated.
    #[error(
        "node {node_id} declared at {declared_at}, more than {max_skew_secs}s ahead of block time {block_time}"
    )]
    DeclarationTooFarInFuture {
        node_id: NodeId,
        declared_at: u64,
        block_time: u64,
        max_skew_secs: u64,
    },

    /// A config value that would leave the contract unable to accept writes.
    #[error("invalid contract configuration: {reason}")]
    InvalidConfig { reason: &'static str },

    /// A storage key's leading subject-class byte matched no known class.
    #[error("unknown subject class tag {tag}")]
    UnknownSubjectClass { tag: u8 },

    /// A measured source's method byte matched no known method.
    #[error("unknown method tag {tag}")]
    UnknownMethod { tag: u8 },

    /// A storage key's trailing source component could not be decoded.
    #[error("malformed source encoding: {0}")]
    InvalidSourceEncoding(String),

    #[error("could not perform contract migration: {comment}")]
    FailedMigration { comment: String },

    /// The referenced node is not a bonded node in the mixnet contract.
    #[error("node {node_id} is not a bonded node in the mixnet contract")]
    NodeNotBonded { node_id: NodeId },

    /// A cross-contract callback was received from a sender other than the
    /// configured mixnet contract.
    #[error("address {sender} is not authorised to invoke the mixnet-contract callback")]
    UnauthorisedMixnetCallback { sender: Addr },

    /// The persisted LtHash digest accumulator was not the expected length, so it
    /// could not be loaded (state corruption - the contract always writes exactly
    /// `nym_lthash::DIGEST_LEN` bytes).
    #[error("the stored digest accumulator is corrupt (unexpected length)")]
    CorruptDigestState,

    #[error(transparent)]
    Admin(#[from] AdminError),

    #[error(transparent)]
    StdErr(#[from] cosmwasm_std::StdError),
}

// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Contract-wide constants.

/// Default maximum length, in bytes, of a payload's `content`. The effective bound lives in
/// contract state and is admin-adjustable; this is only what instantiation starts from,
/// since a later payload version may need more room, or less.
///
/// A bound is needed at all because the contract cannot reject a malformed payload, never
/// parsing one, so it is what keeps the damage a bad value rather than state bloat and an
/// inflated recompute for every verifying client. A single value suffices where the
/// directory contract needs a per-label `max_size`, because the source and subject enums
/// here are closed.
///
/// A realistic version 1 payload is a few hundred bytes of JSON, so this leaves generous
/// headroom while still bounding a batch's total transaction size.
pub const DEFAULT_MAX_PAYLOAD_SIZE: u32 = 1024;

/// Default tolerance, in seconds, for a `declared_at` ahead of block time. Covers worst-case
/// block inclusion plus reasonable clock drift. Without an upper bound at all, one artifact
/// stamped years ahead would freeze a subject's self-declared slot permanently, since nothing
/// could ever exceed it.
pub const DEFAULT_MAX_SKEW_SECS: u64 = 300;

/// Default cap on entries in one batch.
///
/// Chosen to be conservative rather than optimal, and never measured against a chain: at the
/// payload ceiling above, 50 entries are already ~50 KB of content before `Binary`'s base64
/// expansion pushes the message past 60 KB, which bounds a batch by transaction size well
/// before per-entry gas becomes the limit.
///
/// This is only the value instantiation starts from. The effective bound is contract state, so
/// an operator who has measured the real cost on the chain they run against raises or lowers it
/// with [`crate::ExecuteMsg::UpdateConfig`] rather than needing a redeploy.
pub const DEFAULT_MAX_BATCH_SIZE: u32 = 50;

/// The payload version whose `content` is UTF-8 JSON. Never reuse a version for another
/// format; the byte selects the format, not merely the schema.
pub const PAYLOAD_VERSION_1: u8 = 1;

/// Prefix of the bytes a node signs when self-declaring its location.
///
/// Load-bearing, unlike a digest leaf's would be: a node's identity key signs several
/// unrelated message types, and the directory contract's node payload
/// (`node_id || lp(label) || sequence || lp(data)`) also opens with the node id, so without
/// separation a directory signature could be read as a location declaration, its label length
/// and first label bytes landing where `declared_at` is parsed. `MAX_SKEW` happens to reject
/// the timestamps that would produce, but that is an accident of the replay bound rather than
/// a property to rely on.
///
/// The directory's payload carries no tag of its own, so this separation is one-directional.
pub const NYM_NODE_LOCATION_DOMAIN_TAG: &[u8] = b"nym-node-location-declaration-v1";

/// Event names and attribute keys the contract's handlers emit.
pub mod events {
    /// Emitted once per successful measurement batch, not once per entry: a batch runs to
    /// `MAX_BATCH_SIZE` and per-entry events would swamp the block for no gain, since the
    /// entries themselves are queryable.
    pub const SUBMIT_MEASUREMENTS: &str = "submit_measurements";

    /// Emitted once per successful self-declaration relay batch.
    pub const RELAY_SELF_DECLARATIONS: &str = "relay_self_declarations";

    /// Emitted when the admin creates or replaces an override entry.
    pub const SET_OVERRIDE: &str = "set_override";

    /// Emitted when the admin removes an override entry.
    pub const REMOVE_OVERRIDE: &str = "remove_override";

    /// Emitted when the admin deletes a batch of entries by explicit key.
    pub const REMOVE_ENTRIES: &str = "remove_entries";

    /// Emitted when the admin adds an agent or changes an existing agent's permissions.
    pub const SET_WHITELISTED_AGENT: &str = "set_whitelisted_agent";

    /// Emitted when the admin removes an agent from the whitelist.
    pub const REMOVE_WHITELISTED_AGENT: &str = "remove_whitelisted_agent";

    /// Emitted when the admin changes the contract's tunables.
    pub const UPDATE_CONFIG: &str = "update_config";

    /// Emitted when the mixnet unbond callback clears a node's entries.
    pub const ON_NYM_NODE_UNBOND: &str = "on_nym_node_unbond";

    pub const ATTR_AGENT: &str = "agent";
    pub const ATTR_COUNT: &str = "count";
    pub const ATTR_SUBJECT: &str = "subject";

    // the whitelist is the only authorisation a measured entry has, so the grant itself goes in
    // the log: current state is queryable, but who was granted what and when is not
    pub const ATTR_CAN_MEASURE: &str = "can_measure";
    pub const ATTR_CAN_RELAY_SELF_DECLARED: &str = "can_relay_self_declared";

    // likewise the resulting tunables, so a change is auditable after the fact rather than only
    // observable as current state
    pub const ATTR_MAX_SKEW_SECS: &str = "max_skew_secs";
    pub const ATTR_MAX_BATCH_SIZE: &str = "max_batch_size";
    pub const ATTR_MAX_PAYLOAD_SIZE: &str = "max_payload_size";
}

pub mod storage_keys {
    /// `Item<Addr>`: address of the mixnet contract used to validate node existence.
    pub const MIXNET_CONTRACT_ADDRESS: &str = "mixnet-contract-address";

    /// `Admin` (cw-controllers): admin allowed to perform privileged operations.
    pub const CONTRACT_ADMIN: &str = "contract-admin";

    /// The full LtHash accumulator state, `nym_lthash::DIGEST_LEN` raw bytes.
    ///
    /// The one storage key with an external contract. These bytes are what a client obtains an
    /// ICS23 proof for, since CosmWasm smart queries carry none, so three things about this key
    /// are load-bearing and must never change:
    ///
    ///   - it is used **verbatim**, as `store.set(DIGEST_STATE.as_bytes(), ..)`, not through a
    ///     `cw-storage-plus` `Item`. There is no length prefix and no namespacing, so the proven
    ///     key is exactly these bytes appended to the contract's storage prefix (see
    ///     `contract_storage_key` in `nym-validator-client`);
    ///   - the value is the accumulator itself, never its 32-byte collapse. The directory
    ///     contract's client-side digest fetch reads `DIGEST_LEN` bytes here and reconstructs an
    ///     `LtHash16` from them, and would reject a 32-byte value on length;
    ///   - the key never changes, across migrations included. A client pins it in order to prove
    ///     against it, so renaming it silently breaks every verifier rather than failing loudly.
    ///
    /// [`crate::QueryMsg::Digest`] serves the collapse of this same value as an unproven
    /// convenience for consumers that only need to compare digests.
    pub const DIGEST_STATE: &str = "digest_state";

    /// `Item<ContractConfig>`: runtime configuration set at instantiation.
    pub const CONFIG: &str = "config";

    /// `Map<(u8, Vec<u8>, Vec<u8>), LocationEntry>`: location entries, keyed by subject
    /// class, subject id and the flattened source.
    pub const ENTRIES: &str = "entries";

    /// `Map<Addr, AgentPermissions>`: the agent whitelist, a digest-committed entry class
    /// in its own right rather than mere configuration.
    pub const WHITELIST: &str = "whitelist";
}

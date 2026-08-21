// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Contract-wide constants, including storage-key namespaces exposed so off-chain
//! clients can reconstruct the raw keys needed for ICS23 proofs (smart queries
//! do not produce proofs).

/// Hard ceiling on a label's configured `max_size`, in bytes (128 KiB). Guards
/// against an admin fat-finger, on top of the chain's own transaction-size limit.
pub const MAX_LABEL_SIZE_CEILING: u32 = 128 * 1024;

/// Default snapshotting interval, in blocks.
/// With ~6s block time, it works out to around every 10 minutes.
pub const DEFAULT_SNAPSHOT_INTERVAL: u32 = 100;

/// Event types and attribute keys emitted by the contract's execute handlers.
pub mod events {
    /// Emitted on a successful node-entry write.
    pub const SET_NODE_ENTRY: &str = "set_node_entry";
    /// Emitted on a successful node-entry delete.
    pub const DELETE_NODE_ENTRY: &str = "delete_node_entry";
    /// Emitted when the admin adds or updates a whitelisted label.
    pub const SET_LABEL: &str = "set_label";
    /// Emitted when the admin removes a label from the whitelist.
    pub const REMOVE_LABEL: &str = "remove_label";
    /// Emitted on a successful admin curated-entry write.
    pub const SET_CURATED_ENTRY: &str = "set_curated_entry";
    /// Emitted on a successful admin curated-entry delete.
    pub const REMOVE_CURATED_ENTRY: &str = "remove_curated_entry";
    /// Emitted when the mixnet unbond callback clears a node's entries.
    pub const ON_NYM_NODE_UNBOND: &str = "on_nym_node_unbond";

    pub const ATTR_NODE_ID: &str = "node_id";
    pub const ATTR_LABEL: &str = "label";
    pub const ATTR_SEQUENCE: &str = "sequence";
    pub const ATTR_KEY: &str = "key";
    pub const ATTR_MAX_SIZE: &str = "max_size";
}

/// `cw_storage_plus` storage-key namespaces. Kept here (not in the contract crate)
/// so clients can derive raw keys for proofs.
pub mod storage_keys {
    /// `Admin` (cw-controllers): admin allowed to perform privileged operations.
    pub const CONTRACT_ADMIN: &str = "contract-admin";

    /// `Item<u64>`: the directory snapshotting interval, in blocks.
    pub const SNAPSHOT_INTERVAL: &str = "snapshot-interval";

    /// `Item<Addr>`: address of the mixnet contract used to validate node existence.
    pub const MIXNET_CONTRACT_ADDRESS: &str = "mixnet-contract-address";

    /// Node-entry store, keyed `(node_id, label)`. Not a `cw_storage_plus::Map`:
    /// the key is built with `Path`/`Prefix` (so a client can derive raw keys for
    /// proofs) but the value is the compact `NodeEntry` raw-bytes codec, not JSON.
    pub const NODE_ENTRIES: &str = "node_entries";

    /// Curated-entry store, keyed by a single admin-chosen `String` path. Same
    /// `Path`/`Prefix` key handling + raw-bytes value codec as [`NODE_ENTRIES`].
    pub const CURATED_ENTRIES: &str = "curated_entries";

    /// `Map<NodeId, u64>` - the per-node monotonic anti-replay sequence.
    pub const SEQUENCES: &str = "sequences";

    /// `Map<String, LabelConfig>` - the admin-managed label whitelist.
    pub const ALLOWED_LABELS: &str = "allowed_labels";

    /// `Item<[u8; lthash::DIGEST_LEN]>` - the full LtHash accumulator state.
    pub const DIGEST_STATE: &str = "digest_state";
}

// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Contract-wide constants, including storage-key namespaces exposed so off-chain
//! clients can reconstruct the raw keys needed for ICS23 proofs (smart queries
//! do not produce proofs).

/// Hard ceiling on a label's configured `max_size`, in bytes (128 KiB). Guards
/// against an admin fat-finger, on top of the chain's own transaction-size limit.
pub const MAX_LABEL_SIZE_CEILING: u32 = 128 * 1024;

/// `cw_storage_plus` storage-key namespaces. Kept here (not in the contract crate)
/// so clients can derive raw keys for proofs.
pub mod storage_keys {
    /// `Item<Config>`.
    pub const CONFIG: &str = "config";

    /// The single entry store for both classes. Key = `(namespace_tag: u8,
    /// id_bytes, label)`; values are compact raw bytes (see the storage codec).
    pub const ENTRIES: &str = "entries";

    /// `Map<NodeId, u64>` - the per-node monotonic anti-replay sequence.
    pub const SEQUENCES: &str = "sequences";

    /// `Map<String, LabelConfig>` - the admin-managed label whitelist.
    pub const ALLOWED_LABELS: &str = "allowed_labels";

    /// `Item<[u8; lthash::DIGEST_LEN]>` - the full LtHash accumulator state.
    pub const DIGEST_STATE: &str = "digest_state";
}

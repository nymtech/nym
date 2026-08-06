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
/// headroom while still bounding a batch's total transaction size (see task 6.6, which sets
/// `MAX_BATCH_SIZE` from measured gas).
pub const DEFAULT_MAX_PAYLOAD_SIZE: u32 = 1024;

/// Default tolerance, in seconds, for a `declared_at` ahead of block time. Covers worst-case
/// block inclusion plus reasonable clock drift. Without an upper bound at all, one artifact
/// stamped years ahead would freeze a subject's self-declared slot permanently, since nothing
/// could ever exceed it.
pub const DEFAULT_MAX_SKEW_SECS: u64 = 300;

/// Default cap on entries in one batch.
///
/// Provisional: task 6.6 sets this from a measured gas profile against the chain's
/// per-transaction cap, with realistic JSON payloads rather than minimal ones. Treat the
/// current value as a hypothesis, not a measurement.
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

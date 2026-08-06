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
pub const DEFAULT_MAX_PAYLOAD_SIZE: usize = 1024;

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

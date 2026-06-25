// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::DirectoryContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary};
use nym_mixnet_contract_common::NodeId;

/// Key-class / trust-tier discriminant for a directory entry. Extensible: new
/// entry classes get a new variant (and a new, never-reused [`Namespace::tag`]).
#[cw_serde]
#[derive(Copy)]
#[repr(u8)]
pub enum Namespace {
    /// Self-published node entry, authorised by the node's identity key.
    Node = 1,

    /// Admin-curated entry (e.g. a nym-api identity key).
    Curated = 2,
}

impl Namespace {
    /// Stable byte tag identifying the key-class. Used both as the leading byte of
    /// the storage key and in the canonical digest leaf. Never renumber existing
    /// variants (it would re-key/re-hash every existing entry).
    pub const fn tag(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for Namespace {
    type Error = DirectoryContractError;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            n if n == Namespace::Node as u8 => Ok(Namespace::Node),
            n if n == Namespace::Curated as u8 => Ok(Namespace::Curated),
            _ => Err(DirectoryContractError::InvalidNamespace(v)),
        }
    }
}

/// A directory entry of either key-class - the unified in-memory / response type
/// for the single entry store and mixed enumeration; the active variant matches
/// the entry's [`Namespace`]. On-chain the concrete variant is determined by the
/// key's namespace tag, so the raw-bytes value codec stores no redundant
/// discriminant - this `cw_serde` enum is for JSON responses and in-memory use.
#[cw_serde]
pub enum DirectoryEntry {
    /// A self-published node entry.
    NodeEntry(NodeEntry),
    /// An admin-curated entry.
    CuratedEntry(CuratedEntry),
}

/// A node-published entry: opaque bytes, the block height of the last write, and
/// the authoring ed25519 signature (retained for chain-free authorship checks).
#[cw_serde]
pub struct NodeEntry {
    pub data: Binary,
    pub updated_at_height: u64,
    pub signature: Binary,
}

/// An admin-curated entry: opaque bytes (the authority is the contract admin).
#[cw_serde]
pub struct CuratedEntry {
    pub data: Binary,
}

/// Per-label policy.
#[cw_serde]
pub struct LabelConfig {
    /// Maximum permitted `data` length, in bytes, for entries under this label.
    pub max_size: u32,
}

/// Catalog of well-known directory labels. Used by consumers to recognise an
/// entry's label and parse its `data`, and by the contract to seed the initial
/// whitelist at instantiation (every [`KnownLabel::ALL`] variant is auto-whitelisted
/// with its [`KnownLabel::default_config`]).
///
/// Labels are stored on-chain as opaque admin-whitelisted strings, so the admin can
/// add more without a contract migration; this enum itself is a Rust-side catalog,
/// not a serialized type. `#[non_exhaustive]` plus the string mapping mean labels
/// added after a consumer was built fall through as unknown (see
/// [`KnownLabel::from_str`]) and are handled as opaque bytes.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnownLabel {
    /// The node's sphinx keys: a wrapper around two rotation-tagged sphinx (x25519)
    /// keys - either `(previous, current)` or `(current, pre-announced)`. The previous
    /// key's overlap drains long before the next is pre-announced (overlap window <<
    /// the 24h rotation), so three are never held at once; one key at a node's very
    /// first publish. Consumers select by the current rotation; roles are derived,
    /// not stored, so advancement needs no extra writes. Exact payload format TBD.
    SphinxKeys,
}

impl KnownLabel {
    /// Every known label, in a stable order - all auto-whitelisted at contract
    /// instantiation. Keep in sync with the variants above.
    pub const ALL: &'static [KnownLabel] = &[KnownLabel::SphinxKeys];

    /// The canonical on-chain label string for this known label. Stable: once
    /// entries exist under it, the string must not change.
    pub const fn as_str(self) -> &'static str {
        match self {
            KnownLabel::SphinxKeys => "sphinx_key",
        }
    }

    /// The default `max_size` (bytes) this label is whitelisted with at
    /// instantiation; never exceeds [`crate::constants::MAX_LABEL_SIZE_CEILING`].
    pub const fn default_max_size(self) -> u32 {
        match self {
            KnownLabel::SphinxKeys => 256,
        }
    }

    /// The default [`LabelConfig`] used to auto-whitelist this label at instantiation.
    pub const fn default_config(self) -> LabelConfig {
        LabelConfig {
            max_size: self.default_max_size(),
        }
    }
}

/// Returned by [`KnownLabel`]'s [`FromStr`](core::str::FromStr) when the string is
/// not a known label (e.g. an admin-added label this build predates). Carries the
/// unrecognised label value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unrecognised directory label: {0:?}")]
pub struct UnknownLabelError(pub String);

impl core::str::FromStr for KnownLabel {
    type Err = UnknownLabelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sphinx_key" => Ok(KnownLabel::SphinxKeys),
            other => Err(UnknownLabelError(other.to_owned())),
        }
    }
}

/// Contract runtime configuration.
#[cw_serde]
pub struct Config {
    pub mixnet_contract_address: Addr,
}

// ---- query responses ----

/// Response for [`crate::QueryMsg::NodeEntry`]; `None` if the slot is empty.
#[cw_serde]
pub struct NodeEntryResponse {
    pub entry: Option<NodeEntry>,
}

/// Response for [`crate::QueryMsg::CuratedEntry`]; `None` if the slot is empty.
#[cw_serde]
pub struct CuratedEntryResponse {
    pub entry: Option<CuratedEntry>,
}

/// The next sequence a node must sign with (gap-free, exact-match).
#[cw_serde]
pub struct SequenceResponse {
    pub next_sequence: u64,
}

/// The compact 32-byte digest: the BLAKE3 collapse of the LtHash accumulator.
#[cw_serde]
pub struct DigestResponse {
    pub digest: Binary,
}

/// One whitelisted label together with its policy.
#[cw_serde]
pub struct LabelEntry {
    pub label: String,
    pub config: LabelConfig,
}

#[cw_serde]
pub struct AllowedLabelsResponse {
    pub labels: Vec<LabelEntry>,
}

/// A `(label, entry)` pair belonging to a single node.
#[cw_serde]
pub struct NodeLabelEntry {
    pub label: String,
    pub entry: NodeEntry,
}

/// Response for [`crate::QueryMsg::NodeEntries`] - every entry for one node.
#[cw_serde]
pub struct NodeEntriesResponse {
    pub node_id: NodeId,
    pub entries: Vec<NodeLabelEntry>,
}

/// A page of curated entries.
#[cw_serde]
pub struct CuratedEntriesPagedResponse {
    /// `(id, label, entry)` triples in ascending key order.
    pub entries: Vec<(String, String, CuratedEntry)>,
    /// Cursor to pass as the next `start_after`, or `None` when exhausted.
    pub start_next_after: Option<(String, String)>,
}

/// The fully-qualified key of a directory entry in the unified store: its
/// [`Namespace`], the raw id bytes (big-endian `node_id` for the node namespace,
/// handle bytes for curated), and the label.
#[cw_serde]
pub struct EntryKey {
    pub namespace: Namespace,
    pub id: Binary,
    pub label: String,
}

/// One entry together with its key, as yielded by the global enumeration.
#[cw_serde]
pub struct DirectoryEntryRecord {
    pub key: EntryKey,
    pub entry: DirectoryEntry,
}

/// A page of the global entry enumeration across both namespaces - the input from
/// which a client recomputes and verifies the digest.
#[cw_serde]
pub struct AllEntriesPagedResponse {
    pub entries: Vec<DirectoryEntryRecord>,
    /// Cursor to pass as the next `start_after`, or `None` when exhausted.
    pub start_next_after: Option<EntryKey>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::str::FromStr;

    #[test]
    fn known_label_string_round_trip() {
        assert_eq!(KnownLabel::SphinxKeys.as_str(), "sphinx_key");
        assert_eq!(
            KnownLabel::from_str("sphinx_key"),
            Ok(KnownLabel::SphinxKeys)
        );
    }

    #[test]
    fn unknown_label_carries_value() {
        assert_eq!(
            KnownLabel::from_str("not_a_label"),
            Err(UnknownLabelError("not_a_label".to_owned()))
        );
    }

    #[test]
    fn all_known_labels_are_consistent() {
        for &label in KnownLabel::ALL {
            // each catalogued label round-trips through its on-chain string
            assert_eq!(KnownLabel::from_str(label.as_str()), Ok(label));
            // and its auto-whitelist size fits the contract ceiling
            assert!(label.default_max_size() <= crate::constants::MAX_LABEL_SIZE_CEILING);
        }
    }
}

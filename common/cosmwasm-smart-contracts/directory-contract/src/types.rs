// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::DirectoryContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;
use nym_mixnet_contract_common::NodeId;
use std::str::FromStr;

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
    /// Stable byte tag identifying the key-class, used as the leading byte of the
    /// canonical digest leaf so node and curated leaves can never collide. Never
    /// renumber existing variants (it would re-hash every existing entry).
    pub const fn tag(self) -> u8 {
        self as u8
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

impl DirectoryEntry {
    /// The opaque payload bytes, regardless of class.
    pub fn data(&self) -> &[u8] {
        match self {
            DirectoryEntry::NodeEntry(e) => e.data.as_slice(),
            DirectoryEntry::CuratedEntry(e) => e.data.as_slice(),
        }
    }

    /// The compact stored-value encoding for the active variant (see
    /// [`NodeEntry::to_bytes`] / [`CuratedEntry::to_bytes`]). Carries no class
    /// discriminant - the key's [`Namespace`] selects the decoder.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            DirectoryEntry::NodeEntry(e) => e.to_bytes(),
            DirectoryEntry::CuratedEntry(e) => e.to_bytes(),
        }
    }

    /// Decode a stored value, choosing the variant from the key's `namespace`.
    pub fn try_from_bytes(
        namespace: Namespace,
        bytes: &[u8],
    ) -> Result<Self, DirectoryContractError> {
        Ok(match namespace {
            Namespace::Node => DirectoryEntry::NodeEntry(NodeEntry::try_from_bytes(bytes)?),
            Namespace::Curated => {
                DirectoryEntry::CuratedEntry(CuratedEntry::try_from_bytes(bytes)?)
            }
        })
    }

    /// Append this entry's committed value bytes to a digest-leaf buffer. A node
    /// entry commits `data`, `signature`, and `sequence` (so the signature is
    /// independently verifiable); a curated entry commits only `data`. The per-class
    /// field layout is fixed, so the leading namespace tag keeps the two shapes unambiguous.
    fn push_digest_value(&self, buf: &mut Vec<u8>) {
        match self {
            DirectoryEntry::NodeEntry(e) => {
                crate::helpers::push_len_prefixed(buf, e.data.as_slice());
                crate::helpers::push_len_prefixed(buf, e.signature.as_slice());
                buf.extend_from_slice(&e.sequence.to_le_bytes());
            }
            DirectoryEntry::CuratedEntry(e) => {
                crate::helpers::push_len_prefixed(buf, e.data.as_slice());
            }
        }
    }
}

/// A node-published entry: the opaque payload, the block height of the last write,
/// and the `sequence` + ed25519 `signature` it was written with. Storing the
/// sequence makes the signature independently re-verifiable - the signed message is
/// `(node_id, label, sequence, data)` - and both are committed to the digest, so an
/// entry is self-authenticating and the directory is auditable from current state alone.
#[cw_serde]
pub struct NodeEntry {
    pub data: Binary,
    pub updated_at_height: u64,
    pub sequence: u64,
    pub signature: Binary,
}

impl NodeEntry {
    /// Compact value encoding: `updated_at_height || sequence || lp(signature) || data`.
    /// Fixed-width fields first, the variable `signature` length-prefixed, and the
    /// variable `data` as the unframed tail - so no class discriminant is needed
    /// (the storage key's [`Namespace`] tag selects this decoder).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(8 + 8 + 8 + self.signature.len() + self.data.len());
        buf.extend_from_slice(&self.updated_at_height.to_le_bytes());
        buf.extend_from_slice(&self.sequence.to_le_bytes());
        crate::helpers::push_len_prefixed(&mut buf, self.signature.as_slice());
        buf.extend_from_slice(self.data.as_slice());
        buf
    }

    /// Decode the [`Self::to_bytes`] layout.
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, DirectoryContractError> {
        let mut reader = crate::helpers::ValueReader::new(bytes);
        let updated_at_height = reader.read_u64_le()?;
        let sequence = reader.read_u64_le()?;
        let signature = Binary::new(reader.read_len_prefixed()?.to_vec());
        let data = Binary::new(reader.rest().to_vec());
        Ok(NodeEntry {
            data,
            updated_at_height,
            sequence,
            signature,
        })
    }
}

/// An admin-curated entry: opaque bytes (the authority is the contract admin).
#[cw_serde]
pub struct CuratedEntry {
    pub data: Binary,
}

impl CuratedEntry {
    /// Compact value encoding: the raw `data` bytes (its only field).
    pub fn to_bytes(&self) -> Vec<u8> {
        self.data.to_vec()
    }

    /// Decode the [`Self::to_bytes`] layout (the whole buffer is `data`).
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, DirectoryContractError> {
        Ok(CuratedEntry {
            data: Binary::new(bytes.to_vec()),
        })
    }
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

impl FromStr for KnownLabel {
    type Err = UnknownLabelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sphinx_key" => Ok(KnownLabel::SphinxKeys),
            other => Err(UnknownLabelError(other.to_owned())),
        }
    }
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

/// A node entry carrying its full `(node_id, label)` key - a row in
/// [`NodeEntriesPagedResponse`], where entries from different nodes are interleaved.
#[cw_serde]
pub struct AnnotatedNodeLabelEntry {
    pub node_id: NodeId,
    pub label: String,
    pub entry: NodeEntry,
}

/// Response for [`crate::QueryMsg::NodeEntries`] - every entry for one node.
#[cw_serde]
pub struct NodeEntriesResponse {
    pub node_id: NodeId,
    pub entries: Vec<NodeLabelEntry>,
}

/// A `(label, entry)` pair belonging to a curated entry.
#[cw_serde]
pub struct CuratedLabelEntry {
    pub label: String,
    pub entry: CuratedEntry,
}

/// A page of curated entries.
#[cw_serde]
pub struct CuratedEntriesPagedResponse {
    /// Entries in ascending key order.
    pub entries: Vec<CuratedLabelEntry>,
    /// Cursor to pass as the next `start_after`, or `None` when exhausted.
    pub start_next_after: Option<String>,
}

impl From<CuratedEntriesPagedResponse> for AllEntriesPagedResponse {
    fn from(res: CuratedEntriesPagedResponse) -> Self {
        AllEntriesPagedResponse {
            entries: res
                .entries
                .into_iter()
                .map(|curated_entry| {
                    DirectoryEntryRecord::new_curated(curated_entry.label, curated_entry.entry)
                })
                .collect(),
            start_next_after: res.start_next_after.map(EntryKey::new_curated),
        }
    }
}

/// A page of node entries across all nodes, ordered by `(node_id, label)`.
#[cw_serde]
pub struct NodeEntriesPagedResponse {
    /// Entries in ascending `(node_id, label)` order.
    pub entries: Vec<AnnotatedNodeLabelEntry>,
    /// Cursor to pass as the next `start_after`, or `None` when exhausted.
    pub start_next_after: Option<(NodeId, String)>,
}

impl From<NodeEntriesPagedResponse> for AllEntriesPagedResponse {
    fn from(res: NodeEntriesPagedResponse) -> Self {
        AllEntriesPagedResponse {
            entries: res
                .entries
                .into_iter()
                .map(|node_entry| {
                    DirectoryEntryRecord::new_node(
                        node_entry.node_id,
                        node_entry.label,
                        node_entry.entry,
                    )
                })
                .collect(),
            start_next_after: res
                .start_next_after
                .map(|(node_id, label)| EntryKey::new_node(node_id, label)),
        }
    }
}

/// The logical key of a directory entry. Used as the [`crate::QueryMsg::AllEntries`]
/// cursor / response key and to derive the canonical digest leaf; the on-chain
/// storage key is handled separately by each per-class store (via `cw-storage-plus`
/// `Path`/`Prefix`), so this type carries no storage-codec logic.
///
/// - [`EntryKey::Node`] is keyed `(node_id, label)` - node entries are stored under
///   one namespace, so all of a node's entries form a contiguous range (per-node
///   query + unbond cleanup).
/// - [`EntryKey::Curated`] is keyed by a single admin-chosen `key` string under a
///   separate namespace; the admin is responsible for choosing a sensible path
///   (there is no label/suffix structure imposed by the contract).
#[cw_serde]
pub enum EntryKey {
    /// A self-published node entry, keyed `(node_id, label)`.
    Node { node_id: NodeId, label: String },

    /// An admin-curated entry, keyed by a single admin-chosen path string.
    Curated { key: String },
}

impl EntryKey {
    /// A node key from its `(node_id, label)`.
    pub fn new_node(node_id: NodeId, label: String) -> Self {
        EntryKey::Node { node_id, label }
    }

    /// A curated key from its path string.
    pub fn new_curated(key: String) -> Self {
        EntryKey::Curated { key }
    }

    /// The key-class tag for this entry.
    pub fn namespace(&self) -> Namespace {
        match self {
            EntryKey::Node { .. } => Namespace::Node,
            EntryKey::Curated { .. } => Namespace::Curated,
        }
    }

    /// The canonical LtHash leaf for this entry: a class tag, the length-framed key
    /// components, then the entry's committed value. A node leaf commits
    /// `(data, signature, sequence)` so it is self-authenticating; a curated leaf
    /// commits `data`. The leading tag plus length-prefixing make every distinct
    /// `(key, value)` map to distinct leaf bytes, within and across classes.
    pub fn digest_leaf(&self, entry: &DirectoryEntry) -> Vec<u8> {
        let mut buf = vec![self.namespace().tag()];
        match self {
            EntryKey::Node { node_id, label } => {
                // `node_id` is fixed-width, so it needs no length prefix before the
                // variable, length-prefixed `label`.
                buf.extend_from_slice(&node_id.to_be_bytes());
                crate::helpers::push_len_prefixed(&mut buf, label.as_bytes());
            }
            EntryKey::Curated { key } => {
                crate::helpers::push_len_prefixed(&mut buf, key.as_bytes());
            }
        }
        entry.push_digest_value(&mut buf);
        buf
    }
}

/// One entry together with its key, as yielded by the global enumeration.
#[cw_serde]
pub struct DirectoryEntryRecord {
    pub key: EntryKey,
    pub entry: DirectoryEntry,
}

impl DirectoryEntryRecord {
    /// A curated record from its key and entry.
    pub fn new_curated(label: String, entry: CuratedEntry) -> Self {
        Self {
            key: EntryKey::new_curated(label),
            entry: DirectoryEntry::CuratedEntry(entry),
        }
    }

    /// A node record from its `(node_id, label)` and entry.
    pub fn new_node(node_id: NodeId, label: String, entry: NodeEntry) -> Self {
        Self {
            key: EntryKey::new_node(node_id, label),
            entry: DirectoryEntry::NodeEntry(entry),
        }
    }

    /// The canonical LtHash leaf for this record - its key over its committed value.
    pub fn digest_leaf(&self) -> Vec<u8> {
        self.key.digest_leaf(&self.entry)
    }
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

    fn node_entry(data: &[u8], sequence: u64, sig: &[u8]) -> DirectoryEntry {
        DirectoryEntry::NodeEntry(NodeEntry {
            data: data.to_vec().into(),
            updated_at_height: 0,
            sequence,
            signature: sig.to_vec().into(),
        })
    }

    fn curated_entry(data: &[u8]) -> DirectoryEntry {
        DirectoryEntry::CuratedEntry(CuratedEntry {
            data: data.to_vec().into(),
        })
    }

    #[test]
    fn digest_leaf_classes_differ() {
        // same string + data, different class -> different leaf (the tag separates them)
        let node_key = EntryKey::Node {
            node_id: 1,
            label: "x".into(),
        };
        let curated_key = EntryKey::Curated { key: "x".into() };
        assert_ne!(
            node_key.digest_leaf(&node_entry(b"v", 0, b"sig")),
            curated_key.digest_leaf(&curated_entry(b"v")),
        );
    }

    #[test]
    fn digest_leaf_length_prefix_disambiguates() {
        // curated (key "ab", data "c") vs (key "a", data "bc") must not collide
        let ab_c = EntryKey::Curated { key: "ab".into() };
        let a_bc = EntryKey::Curated { key: "a".into() };
        assert_ne!(
            ab_c.digest_leaf(&curated_entry(b"c")),
            a_bc.digest_leaf(&curated_entry(b"bc")),
        );
        // and likewise for a node's (label, data) framing
        let node_ab = EntryKey::Node {
            node_id: 1,
            label: "ab".into(),
        };
        let node_a = EntryKey::Node {
            node_id: 1,
            label: "a".into(),
        };
        assert_ne!(
            node_ab.digest_leaf(&node_entry(b"c", 0, b"s")),
            node_a.digest_leaf(&node_entry(b"bc", 0, b"s")),
        );
    }

    #[test]
    fn node_leaf_commits_signature_and_sequence() {
        let key = EntryKey::Node {
            node_id: 1,
            label: "x".into(),
        };
        let base = key.digest_leaf(&node_entry(b"d", 5, b"sigA"));
        assert_eq!(
            base,
            key.digest_leaf(&node_entry(b"d", 5, b"sigA")),
            "deterministic"
        );
        assert_ne!(
            base,
            key.digest_leaf(&node_entry(b"d", 5, b"sigB")),
            "signature is committed"
        );
        assert_ne!(
            base,
            key.digest_leaf(&node_entry(b"d", 6, b"sigA")),
            "sequence is committed"
        );
    }

    #[test]
    fn node_leaf_excludes_updated_at_height() {
        let key = EntryKey::Node {
            node_id: 1,
            label: "x".into(),
        };
        let a = DirectoryEntry::NodeEntry(NodeEntry {
            data: b"d".to_vec().into(),
            updated_at_height: 1,
            sequence: 5,
            signature: b"s".to_vec().into(),
        });
        let b = DirectoryEntry::NodeEntry(NodeEntry {
            data: b"d".to_vec().into(),
            updated_at_height: 999,
            sequence: 5,
            signature: b"s".to_vec().into(),
        });
        assert_eq!(
            key.digest_leaf(&a),
            key.digest_leaf(&b),
            "height must not be committed"
        );
    }

    #[test]
    fn node_entry_value_round_trips() {
        let entry = NodeEntry {
            data: b"opaque payload".to_vec().into(),
            updated_at_height: 123_456,
            sequence: 7,
            signature: vec![9u8; 64].into(),
        };
        let bytes = entry.to_bytes();
        assert_eq!(NodeEntry::try_from_bytes(&bytes), Ok(entry));
    }

    #[test]
    fn node_entry_value_handles_empty_data_and_signature() {
        let entry = NodeEntry {
            data: Binary::default(),
            updated_at_height: 0,
            sequence: 0,
            signature: Binary::default(),
        };
        let bytes = entry.to_bytes();
        assert_eq!(NodeEntry::try_from_bytes(&bytes), Ok(entry));
    }

    #[test]
    fn curated_entry_value_round_trips() {
        for data in [b"".as_slice(), b"x", b"a longer curated payload"] {
            let entry = CuratedEntry {
                data: data.to_vec().into(),
            };
            assert_eq!(CuratedEntry::try_from_bytes(&entry.to_bytes()), Ok(entry));
        }
    }

    #[test]
    fn directory_entry_value_dispatches_on_namespace() {
        let node = node_entry(b"d", 3, b"sig");
        let decoded = DirectoryEntry::try_from_bytes(Namespace::Node, &node.to_bytes());
        assert_eq!(decoded, Ok(node));

        let curated = curated_entry(b"c");
        let decoded = DirectoryEntry::try_from_bytes(Namespace::Curated, &curated.to_bytes());
        assert_eq!(decoded, Ok(curated));
    }

    #[test]
    fn node_entry_value_rejects_truncation() {
        // a 10-byte buffer cannot hold the two u64 fields + a length prefix
        assert!(NodeEntry::try_from_bytes(&[0u8; 10]).is_err());
        assert!(NodeEntry::try_from_bytes(&[]).is_err());
        // a length prefix that overruns the remaining bytes is rejected
        let mut bad = Vec::new();
        bad.extend_from_slice(&0u64.to_le_bytes()); // updated_at_height
        bad.extend_from_slice(&0u64.to_le_bytes()); // sequence
        bad.extend_from_slice(&99u64.to_le_bytes()); // signature length (overruns)
        assert!(NodeEntry::try_from_bytes(&bad).is_err());
    }
}

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

impl DirectoryEntry {
    /// The opaque payload bytes, regardless of class.
    pub fn data(&self) -> &[u8] {
        match self {
            DirectoryEntry::NodeEntry(e) => e.data.as_slice(),
            DirectoryEntry::CuratedEntry(e) => e.data.as_slice(),
        }
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

/// Response for [`crate::QueryMsg::NodeEntries`] - every entry for one node.
#[cw_serde]
pub struct NodeEntriesResponse {
    pub node_id: NodeId,
    pub entries: Vec<NodeLabelEntry>,
}

/// The key of a curated entry: its label plus an optional instance `suffix`.
#[cw_serde]
pub struct CuratedKey {
    pub label: String,
    pub suffix: Option<String>,
}

/// A page of curated entries.
#[cw_serde]
pub struct CuratedEntriesPagedResponse {
    /// `(key, entry)` pairs in ascending key order.
    pub entries: Vec<(CuratedKey, CuratedEntry)>,
    /// Cursor to pass as the next `start_after`, or `None` when exhausted.
    pub start_next_after: Option<CuratedKey>,
}

/// The fully-qualified key of a directory entry in the unified store. Each class
/// orders its key for its own access pattern, so the byte layout is per-variant
/// (but always tagged and fully length-prefixed in the digest leaf, so leaves can
/// never collide):
///
/// - [`EntryKey::Node`] is keyed `(node_id, label)` - the node leads, so all of one
///   node's entries form a contiguous range (used by the unbond cleanup and the
///   per-node query). `node_id` is mandatory.
/// - [`EntryKey::Curated`] is keyed `(label, suffix)` - the label leads, so all
///   instances of a label (e.g. every `nym-api`) form a contiguous range. `suffix`
///   is optional (`None` is a singleton under the label).
///
/// The canonical encodings (storage key and digest leaf) are derived here so the
/// contract and any off-chain client cannot disagree on them.
#[cw_serde]
pub enum EntryKey {
    /// A self-published node entry, keyed `(node_id, label)`.
    Node { node_id: NodeId, label: String },

    /// An admin-curated entry, keyed `(label, suffix)`. `suffix` distinguishes
    /// multiple instances of one label (e.g. label `"nym-api"`, suffix `"foo"`);
    /// `None` is a singleton keyed by the label alone. When `Some`, the suffix MUST
    /// be non-empty - an empty suffix is indistinguishable from `None` in the key
    /// and is rejected by the contract.
    Curated {
        label: String,
        suffix: Option<String>,
    },
}

impl EntryKey {
    /// The key-class tag for this entry.
    pub fn namespace(&self) -> Namespace {
        match self {
            EntryKey::Node { .. } => Namespace::Node,
            EntryKey::Curated { .. } => Namespace::Curated,
        }
    }

    /// The label component, common to every class.
    pub fn label(&self) -> &str {
        match self {
            EntryKey::Node { label, .. } | EntryKey::Curated { label, .. } => label,
        }
    }

    /// The `(leading, trailing)` key components in canonical order: `(node_id,
    /// label)` for a node, `(label, suffix)` for a curated entry. The leading part
    /// is length-prefixed (in both the storage key and the digest leaf); the
    /// trailing part is the final segment, so the leading group is a contiguous
    /// range.
    fn key_parts(&self) -> (Vec<u8>, Vec<u8>) {
        match self {
            EntryKey::Node { node_id, label } => {
                (node_id.to_be_bytes().to_vec(), label.as_bytes().to_vec())
            }
            EntryKey::Curated { label, suffix } => (
                label.as_bytes().to_vec(),
                suffix
                    .as_deref()
                    .map(|s| s.as_bytes().to_vec())
                    .unwrap_or_default(),
            ),
        }
    }

    /// `tag || len_prefixed(leading)` - the prefix shared by every entry with the
    /// same `(namespace, leading)`. Every full [`Self::storage_key`] begins with it,
    /// so it doubles as the range-scan prefix for that group.
    fn class_leading_prefix(namespace: Namespace, leading: &[u8]) -> Vec<u8> {
        let mut buf = vec![namespace.tag()];
        crate::helpers::push_len_prefixed(&mut buf, leading);
        buf
    }

    /// The raw storage key: `tag || len_prefixed(leading) || trailing`.
    pub fn storage_key(&self) -> Vec<u8> {
        let (leading, trailing) = self.key_parts();
        let mut buf = Self::class_leading_prefix(self.namespace(), &leading);
        buf.extend_from_slice(&trailing);
        buf
    }

    /// Parse a raw key produced by [`Self::storage_key`] back into an `EntryKey`.
    /// Off-chain clients use this to decode keys returned in ICS23 proofs.
    pub fn from_storage_key(bytes: &[u8]) -> Result<Self, DirectoryContractError> {
        fn malformed(m: &str) -> DirectoryContractError {
            DirectoryContractError::MalformedStorageKey(m.to_owned())
        }
        fn utf8(b: &[u8]) -> Result<String, DirectoryContractError> {
            String::from_utf8(b.to_vec()).map_err(|_| malformed("non-UTF-8 label or suffix"))
        }

        let (&tag, rest) = bytes.split_first().ok_or_else(|| malformed("empty key"))?;
        let namespace = Namespace::try_from(tag)?;

        if rest.len() < 8 {
            return Err(malformed("truncated length prefix"));
        }
        let (len_bytes, rest) = rest.split_at(8);

        // SAFETY: we have checked we have at least 8 bytes
        #[allow(clippy::unwrap_used)]
        let len = u64::from_le_bytes(len_bytes.try_into().unwrap()) as usize;
        if rest.len() < len {
            return Err(malformed("leading length exceeds key"));
        }
        let (leading, trailing) = rest.split_at(len);

        match namespace {
            Namespace::Node => {
                let id: [u8; 4] = leading
                    .try_into()
                    .map_err(|_| malformed("node id must be 4 bytes"))?;
                Ok(EntryKey::Node {
                    node_id: NodeId::from_be_bytes(id),
                    label: utf8(trailing)?,
                })
            }
            Namespace::Curated => Ok(EntryKey::Curated {
                label: utf8(leading)?,
                suffix: (!trailing.is_empty()).then(|| utf8(trailing)).transpose()?,
            }),
        }
    }

    /// Range-scan prefix for an entire key-class.
    pub fn namespace_prefix(namespace: Namespace) -> Vec<u8> {
        vec![namespace.tag()]
    }

    /// Range-scan prefix for all of one node's entries (per-node query + unbond
    /// cleanup). Guaranteed to be a prefix of every node `storage_key`.
    pub fn node_prefix(node_id: NodeId) -> Vec<u8> {
        Self::class_leading_prefix(Namespace::Node, &node_id.to_be_bytes())
    }

    /// Range-scan prefix for every instance under one curated label. Guaranteed to
    /// be a prefix of every curated `storage_key` with that label.
    pub fn curated_label_prefix(label: &str) -> Vec<u8> {
        Self::class_leading_prefix(Namespace::Curated, label.as_bytes())
    }

    /// The committed key bytes: `tag || len_prefixed(leading) || len_prefixed(trailing)`.
    fn digest_key_prefix(&self) -> Vec<u8> {
        let (leading, trailing) = self.key_parts();
        let mut buf = Self::class_leading_prefix(self.namespace(), &leading);
        crate::helpers::push_len_prefixed(&mut buf, &trailing);
        buf
    }

    /// The canonical LtHash leaf for this entry: the committed key bytes followed by
    /// the entry's committed value. A node leaf commits `(data, signature, sequence)`
    /// so it is self-authenticating; a curated leaf commits `data`.
    /// Fully length-prefixed, so no two distinct entries can hash to the same leaf.
    pub fn digest_leaf(&self, entry: &DirectoryEntry) -> Vec<u8> {
        let mut buf = self.digest_key_prefix();
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
        // same label + data, different class -> different leaf (the tag separates them)
        let node_key = EntryKey::Node {
            node_id: 1,
            label: "x".into(),
        };
        let curated_key = EntryKey::Curated {
            label: "x".into(),
            suffix: Some("1".into()),
        };
        assert_ne!(
            node_key.digest_leaf(&node_entry(b"v", 0, b"sig")),
            curated_key.digest_leaf(&curated_entry(b"v")),
        );
    }

    #[test]
    fn digest_leaf_length_prefix_disambiguates() {
        // (label "ab", suffix "c") vs (label "a", suffix "bc") must not collide
        let ab_c = EntryKey::Curated {
            label: "ab".into(),
            suffix: Some("c".into()),
        };
        let a_bc = EntryKey::Curated {
            label: "a".into(),
            suffix: Some("bc".into()),
        };
        assert_ne!(
            ab_c.digest_leaf(&curated_entry(b"")),
            a_bc.digest_leaf(&curated_entry(b"")),
        );
    }

    #[test]
    fn curated_singleton_and_instance_differ() {
        // a singleton (None) and a suffixed instance under the same label are distinct
        let singleton = EntryKey::Curated {
            label: "x".into(),
            suffix: None,
        };
        let instance = EntryKey::Curated {
            label: "x".into(),
            suffix: Some("foo".into()),
        };
        assert_ne!(singleton.storage_key(), instance.storage_key());
        assert_ne!(
            singleton.digest_leaf(&curated_entry(b"v")),
            instance.digest_leaf(&curated_entry(b"v")),
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
    fn storage_key_round_trips() {
        for key in [
            EntryKey::Node {
                node_id: 42,
                label: "sphinx_key".into(),
            },
            EntryKey::Node {
                node_id: 0,
                label: String::new(),
            },
            EntryKey::Curated {
                label: "nym-api".into(),
                suffix: Some("foo".into()),
            },
            EntryKey::Curated {
                label: "singleton".into(),
                suffix: None,
            },
        ] {
            let bytes = key.storage_key();
            assert_eq!(EntryKey::from_storage_key(&bytes), Ok(key));
        }
    }

    #[test]
    fn from_storage_key_rejects_garbage() {
        assert!(EntryKey::from_storage_key(&[]).is_err()); // empty
        assert!(EntryKey::from_storage_key(&[9]).is_err()); // unknown namespace tag
        assert!(EntryKey::from_storage_key(&[Namespace::Node.tag(), 0, 0]).is_err()); // truncated length
        // node leading must be exactly 4 bytes
        let mut k = vec![Namespace::Node.tag()];
        k.extend_from_slice(&3u64.to_le_bytes());
        k.extend_from_slice(b"abc");
        assert!(EntryKey::from_storage_key(&k).is_err());
    }

    #[test]
    fn scan_prefixes_are_prefixes_of_their_keys() {
        let node = EntryKey::Node {
            node_id: 7,
            label: "l".into(),
        };
        assert!(node.storage_key().starts_with(&EntryKey::node_prefix(7)));
        assert!(
            node.storage_key()
                .starts_with(&EntryKey::namespace_prefix(Namespace::Node))
        );
        // a different node is not under node 7's prefix
        let other = EntryKey::Node {
            node_id: 8,
            label: "l".into(),
        };
        assert!(!other.storage_key().starts_with(&EntryKey::node_prefix(7)));

        let curated = EntryKey::Curated {
            label: "nym-api".into(),
            suffix: Some("foo".into()),
        };
        assert!(
            curated
                .storage_key()
                .starts_with(&EntryKey::curated_label_prefix("nym-api"))
        );
        // a different label is not under "nym-api"'s prefix
        let elsewhere = EntryKey::Curated {
            label: "nym-apiz".into(),
            suffix: None,
        };
        assert!(
            !elsewhere
                .storage_key()
                .starts_with(&EntryKey::curated_label_prefix("nym-api"))
        );
    }

    #[test]
    fn prefix_upper_bound_brackets_the_prefix() {
        use crate::helpers::prefix_upper_bound;
        let prefix = EntryKey::node_prefix(7);
        let upper = prefix_upper_bound(&prefix).expect("prefix is not all-0xff");
        // node 7's keys fall within [prefix, upper)
        let in_range = EntryKey::Node {
            node_id: 7,
            label: "z".into(),
        }
        .storage_key();
        assert!(prefix.as_slice() <= in_range.as_slice());
        assert!(in_range.as_slice() < upper.as_slice());
        // node 8 sits at or beyond the upper bound
        let beyond = EntryKey::Node {
            node_id: 8,
            label: String::new(),
        }
        .storage_key();
        assert!(beyond.as_slice() >= upper.as_slice());
        // an all-0xff prefix has no upper bound
        assert_eq!(prefix_upper_bound(&[0xff, 0xff]), None);
    }
}

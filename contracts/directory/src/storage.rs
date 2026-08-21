// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, DepsMut, Order, Record, Storage};
use cw_controllers::Admin;
use cw_storage_plus::{
    range_with_prefix, Bound, Item, Key, KeyDeserialize, Map, Path, Prefix, PrimaryKey,
};
use nym_directory_contract_common::constants::storage_keys;
use nym_directory_contract_common::msg::InitialLabel;
use nym_directory_contract_common::{
    CuratedEntry, DirectoryContractError, DirectoryEntryRecord, KnownLabel, LabelConfig, NodeEntry,
};
use nym_lthash::LtHash16;
use nym_mixnet_contract_common::NodeId;
use std::ops::Deref;

pub const NYM_DIRECTORY_CONTRACT_STORAGE: NymDirectoryContractStorage =
    NymDirectoryContractStorage::new();

pub struct NymDirectoryContractStorage {
    /// Admin of the contract; gates privileged operations.
    pub(crate) contract_admin: Admin,

    /// Address of the mixnet contract; used to verify a node id refers to a
    /// real, registered, and bonded node.
    pub(crate) mixnet_contract_address: Item<Addr>,

    pub(crate) sequences: Map<NodeId, u64>,

    pub(crate) allowed_storage_labels: Map<String, LabelConfig>,

    // The LtHash digest accumulator (~2 KB) is NOT a `cw-storage-plus` `Item`: serde
    // cannot (de)serialize a `[u8; DIGEST_LEN]` (it only derives arrays up to len 32),
    // and base64-encoding it on every write would be wasteful. It is stored raw under
    // `storage_keys::DIGEST_STATE` via `load_digest` / `save_digest` below.
    /// Self-published node entries, keyed `(node_id, label)`.
    pub(crate) node_entries: StoredNodeEntries,

    /// Admin-curated entries, keyed by a single admin-chosen path string.
    pub(crate) curated_entries: StoredCuratedEntries,
}

impl NymDirectoryContractStorage {
    #[allow(clippy::new_without_default)]
    pub(crate) const fn new() -> Self {
        NymDirectoryContractStorage {
            contract_admin: Admin::new(storage_keys::CONTRACT_ADMIN),
            mixnet_contract_address: Item::new(storage_keys::MIXNET_CONTRACT_ADDRESS),
            sequences: Map::new(storage_keys::SEQUENCES),
            allowed_storage_labels: Map::new(storage_keys::ALLOWED_LABELS),
            node_entries: StoredNodeEntries,
            curated_entries: StoredCuratedEntries,
        }
    }

    /// One-time storage initialisation called from the contract's `instantiate`
    /// entry point. Persists the mixnet contract address
    /// and sets `sender` as the contract admin.
    pub(crate) fn initialise(
        &self,
        deps: DepsMut,
        sender: Addr,
        mixnet_contract_address: Addr,
        initial_labels: Vec<InitialLabel>,
    ) -> Result<(), DirectoryContractError> {
        // 1. set mixnet contract address
        self.mixnet_contract_address
            .save(deps.storage, &mixnet_contract_address)?;

        // 2. save known labels
        for label in KnownLabel::ALL {
            self.allowed_storage_labels.save(
                deps.storage,
                label.as_str().to_string(),
                &label.default_config(),
            )?;
        }

        // 3. save additional, provided, labels (if applicable)
        // if there's an overlap with the known labels,
        // prefer the explicitly provided config
        for initial_label in initial_labels {
            self.allowed_storage_labels.save(
                deps.storage,
                initial_label.label.as_str().to_string(),
                &initial_label.config,
            )?;
        }

        // 4. save contract admin (consumes deps)
        self.contract_admin.set(deps, Some(sender))?;

        Ok(())
    }

    pub(crate) fn current_sequence(
        &self,
        store: &dyn Storage,
        node_id: NodeId,
    ) -> Result<u64, DirectoryContractError> {
        Ok(self.sequences.may_load(store, node_id)?.unwrap_or_default())
    }

    pub(crate) fn save_account_sequence(
        &self,
        store: &mut dyn Storage,
        node_id: NodeId,
        new_sequence: u64,
    ) -> Result<(), DirectoryContractError> {
        self.sequences.save(store, node_id, &new_sequence)?;
        Ok(())
    }

    // ---- digest accumulator (raw `DIGEST_STATE` key) ----

    /// Load the global LtHash accumulator, or the empty digest if nothing has been
    /// written yet.
    pub(crate) fn load_digest(
        &self,
        store: &dyn Storage,
    ) -> Result<LtHash16, DirectoryContractError> {
        match store.get(storage_keys::DIGEST_STATE.as_bytes()) {
            Some(bytes) => {
                let raw: &[u8; nym_lthash::DIGEST_LEN] = bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| DirectoryContractError::CorruptDigestState)?;
                Ok(LtHash16::from_bytes(raw))
            }
            None => Ok(LtHash16::new()),
        }
    }

    fn save_digest(&self, store: &mut dyn Storage, digest: &LtHash16) {
        store.set(storage_keys::DIGEST_STATE.as_bytes(), &digest.to_bytes());
    }

    // ---- digest-maintaining entry mutations ----
    //
    // Every write/delete keeps the global multiset digest in sync: the old entry's
    // leaf (if any) is subtracted and the new entry's leaf added, so the digest
    // always equals the LtHash over the current entry set. The leaf is the canonical
    // `EntryKey::digest_leaf` (independent of storage layout). Entries are taken by
    // value so the new leaf is computed by moving (not cloning) the entry after it
    // has been persisted by reference.

    /// Create or replace a node entry, keeping the digest in sync.
    pub(crate) fn set_node_entry(
        &self,
        store: &mut dyn Storage,
        node_id: NodeId,
        label: &str,
        entry: NodeEntry,
    ) -> Result<(), DirectoryContractError> {
        let mut digest = self.load_digest(store)?;

        // replacing an existing entry: retire its old leaf first
        if let Some(old) = self.node_entries.may_load(store, node_id, label)? {
            digest.subtract(
                &DirectoryEntryRecord::new_node(node_id, label.to_owned(), old).digest_leaf(),
            );
        }

        self.node_entries.save(store, node_id, label, &entry);
        digest.add(&DirectoryEntryRecord::new_node(node_id, label.to_owned(), entry).digest_leaf());
        self.save_digest(store, &digest);
        Ok(())
    }

    /// Delete a node entry, keeping the digest in sync. No-op (digest untouched) if
    /// the entry does not exist.
    pub(crate) fn remove_node_entry(
        &self,
        store: &mut dyn Storage,
        node_id: NodeId,
        label: &str,
    ) -> Result<(), DirectoryContractError> {
        let Some(old) = self.node_entries.may_load(store, node_id, label)? else {
            return Ok(());
        };
        let mut digest = self.load_digest(store)?;
        digest.subtract(
            &DirectoryEntryRecord::new_node(node_id, label.to_owned(), old).digest_leaf(),
        );
        self.node_entries.remove(store, node_id, label);
        self.save_digest(store, &digest);
        Ok(())
    }

    /// Delete ALL of one node's entries (every label) in a single digest update -
    /// the unbond-cleanup path. Loads/saves the accumulator once and subtracts every
    /// leaf, rather than paying a digest load+save per label. Idempotent: a node with
    /// no entries leaves the digest untouched. Bounded by the (governed) label set.
    pub(crate) fn remove_all_node_entries(
        &self,
        store: &mut dyn Storage,
        node_id: NodeId,
    ) -> Result<(), DirectoryContractError> {
        // Collect first: the scan borrows the store immutably, and we then mutate it.
        let entries = self
            .node_entries
            .node_range(store, node_id)
            .collect::<Result<Vec<_>, _>>()?;

        if entries.is_empty() {
            return Ok(());
        }

        let mut digest = self.load_digest(store)?;
        for (label, entry) in entries {
            digest.subtract(
                &DirectoryEntryRecord::new_node(node_id, label.clone(), entry).digest_leaf(),
            );
            self.node_entries.remove(store, node_id, &label);
        }
        self.save_digest(store, &digest);
        Ok(())
    }

    /// Create or replace a curated entry, keeping the digest in sync.
    pub(crate) fn set_curated_entry(
        &self,
        store: &mut dyn Storage,
        key: &str,
        entry: CuratedEntry,
    ) -> Result<(), DirectoryContractError> {
        let mut digest = self.load_digest(store)?;

        if let Some(old) = self.curated_entries.may_load(store, key)? {
            digest.subtract(&DirectoryEntryRecord::new_curated(key.to_owned(), old).digest_leaf());
        }

        self.curated_entries.save(store, key, &entry);
        digest.add(&DirectoryEntryRecord::new_curated(key.to_owned(), entry).digest_leaf());
        self.save_digest(store, &digest);
        Ok(())
    }

    /// Delete a curated entry, keeping the digest in sync. No-op (digest untouched)
    /// if the entry does not exist.
    pub(crate) fn remove_curated_entry(
        &self,
        store: &mut dyn Storage,
        key: &str,
    ) -> Result<(), DirectoryContractError> {
        let Some(old) = self.curated_entries.may_load(store, key)? else {
            return Ok(());
        };
        let mut digest = self.load_digest(store)?;
        digest.subtract(&DirectoryEntryRecord::new_curated(key.to_owned(), old).digest_leaf());
        self.curated_entries.remove(store, key);
        self.save_digest(store, &digest);
        Ok(())
    }
}

/// Raw-bytes reader/writer for the node-entry store, keyed `(node_id, label)`.
/// Mirrors the ecash `StoredDeposits` pattern: `cw-storage-plus` owns the key
/// (`Path` for writes, `Prefix` + `KeyDeserialize` for scans, so a client can still
/// reconstruct the raw key for an ICS23 proof) while the value is the compact
/// [`NodeEntry::to_bytes`] codec instead of JSON.
pub(crate) struct StoredNodeEntries;

impl StoredNodeEntries {
    const NAMESPACE: &'static [u8] = storage_keys::NODE_ENTRIES.as_bytes();

    /// The raw storage key for `(node_id, label)`.
    fn storage_key(node_id: NodeId, label: &str) -> Path<Vec<u8>> {
        let key = (node_id, label.to_owned());
        let parts = key.key();
        Path::new(
            Self::NAMESPACE,
            &parts.iter().map(Key::as_ref).collect::<Vec<_>>(),
        )
    }

    /// Scan prefix covering the whole store.
    fn no_prefix() -> Prefix<(NodeId, String), NodeEntry> {
        Prefix::new(Self::NAMESPACE, &[])
    }

    /// Scan prefix covering one node's contiguous `(node_id, *)` range.
    fn node_prefix(node_id: NodeId) -> Prefix<String, NodeEntry> {
        Prefix::new(Self::NAMESPACE, &node_id.key())
    }

    fn decode(kv: Record) -> Result<((NodeId, String), NodeEntry), DirectoryContractError> {
        let (k, v) = kv;
        let key = <(NodeId, String) as KeyDeserialize>::from_vec(k)?;
        Ok((key, NodeEntry::try_from_bytes(&v)?))
    }

    fn decode_label(kv: Record) -> Result<(String, NodeEntry), DirectoryContractError> {
        let (k, v) = kv;
        let label = <String as KeyDeserialize>::from_vec(k)?;
        Ok((label, NodeEntry::try_from_bytes(&v)?))
    }

    /// Create or overwrite a node entry.
    pub(crate) fn save(
        &self,
        store: &mut dyn Storage,
        node_id: NodeId,
        label: &str,
        entry: &NodeEntry,
    ) {
        store.set(&Self::storage_key(node_id, label), &entry.to_bytes());
    }

    /// Load a single node entry; `None` if the slot is empty.
    pub(crate) fn may_load(
        &self,
        store: &dyn Storage,
        node_id: NodeId,
        label: &str,
    ) -> Result<Option<NodeEntry>, DirectoryContractError> {
        match store.get(&Self::storage_key(node_id, label)) {
            Some(bytes) => Ok(Some(NodeEntry::try_from_bytes(&bytes)?)),
            None => Ok(None),
        }
    }

    /// Delete a node entry. Idempotent - removing an absent key is a no-op.
    pub(crate) fn remove(&self, store: &mut dyn Storage, node_id: NodeId, label: &str) {
        store.remove(&Self::storage_key(node_id, label));
    }

    /// Range over the whole node store, ordered by `(node_id, label)`. The bound is
    /// the full composite key (a `String`-only bound would be compared against keys
    /// led by the node-id length prefix and so would not mean "after this label").
    pub(crate) fn range<'a>(
        &'a self,
        store: &'a dyn Storage,
        min: Option<Bound<'a, (NodeId, String)>>,
        max: Option<Bound<'a, (NodeId, String)>>,
        order: Order,
    ) -> impl Iterator<Item = Result<((NodeId, String), NodeEntry), DirectoryContractError>> + 'a
    {
        let prefix = Self::no_prefix();
        let mapped = range_with_prefix(
            store,
            prefix.deref(),
            min.map(|b| b.to_raw_bound()),
            max.map(|b| b.to_raw_bound()),
            order,
        )
        .map(Self::decode);
        Box::new(mapped)
    }

    /// All of one node's entries (its contiguous label range), unpaginated. Backs the
    /// per-node query and the unbond cleanup.
    pub(crate) fn node_range<'a>(
        &'a self,
        store: &'a dyn Storage,
        node_id: NodeId,
    ) -> impl Iterator<Item = Result<(String, NodeEntry), DirectoryContractError>> + 'a {
        let prefix = Self::node_prefix(node_id);
        let mapped = range_with_prefix(store, prefix.deref(), None, None, Order::Ascending)
            .map(Self::decode_label);
        Box::new(mapped)
    }
}

/// Raw-bytes reader/writer for the curated-entry store, keyed by a single
/// admin-chosen path `String`. Same `Path`/`Prefix` key handling and compact
/// [`CuratedEntry::to_bytes`] value codec as [`StoredNodeEntries`].
pub(crate) struct StoredCuratedEntries;

impl StoredCuratedEntries {
    const NAMESPACE: &'static [u8] = storage_keys::CURATED_ENTRIES.as_bytes();

    fn storage_key(key: &str) -> Path<Vec<u8>> {
        let parts = key.key();
        Path::new(
            Self::NAMESPACE,
            &parts.iter().map(Key::as_ref).collect::<Vec<_>>(),
        )
    }

    fn no_prefix() -> Prefix<String, CuratedEntry> {
        Prefix::new(Self::NAMESPACE, &[])
    }

    fn decode(kv: Record) -> Result<(String, CuratedEntry), DirectoryContractError> {
        let (k, v) = kv;
        let key = <String as KeyDeserialize>::from_vec(k)?;
        Ok((key, CuratedEntry::try_from_bytes(&v)?))
    }

    /// Create or overwrite a curated entry.
    pub(crate) fn save(&self, store: &mut dyn Storage, key: &str, entry: &CuratedEntry) {
        store.set(&Self::storage_key(key), &entry.to_bytes());
    }

    /// Load a single curated entry; `None` if the slot is empty.
    pub(crate) fn may_load(
        &self,
        store: &dyn Storage,
        key: &str,
    ) -> Result<Option<CuratedEntry>, DirectoryContractError> {
        match store.get(&Self::storage_key(key)) {
            Some(bytes) => Ok(Some(CuratedEntry::try_from_bytes(&bytes)?)),
            None => Ok(None),
        }
    }

    /// Delete a curated entry. Idempotent - removing an absent key is a no-op.
    pub(crate) fn remove(&self, store: &mut dyn Storage, key: &str) {
        store.remove(&Self::storage_key(key));
    }

    pub(crate) fn range<'a>(
        &'a self,
        store: &'a dyn Storage,
        min: Option<Bound<'a, String>>,
        max: Option<Bound<'a, String>>,
        order: Order,
    ) -> impl Iterator<Item = Result<(String, CuratedEntry), DirectoryContractError>> + 'a {
        let prefix = Self::no_prefix();
        let mapped = range_with_prefix(
            store,
            prefix.deref(),
            min.map(|b| b.to_raw_bound()),
            max.map(|b| b.to_raw_bound()),
            order,
        )
        .map(Self::decode);
        Box::new(mapped)
    }
}

pub mod retrieval_limits {
    pub const DEFAULT_CURATED_ENTRIES: u32 = 50;
    pub const MAX_CURATED_ENTRIES: u32 = 100;

    pub const DEFAULT_NODE_ENTRIES: u32 = 50;
    pub const MAX_NODE_ENTRIES: u32 = 100;

    pub const DEFAULT_ALL_ENTRIES: u32 = 50;
    pub const MAX_ALL_ENTRIES: u32 = 100;

    // the below must hold otherwise `query_all_entries` will fail
    const _: () = assert!(MAX_NODE_ENTRIES >= MAX_ALL_ENTRIES);
    const _: () = assert!(DEFAULT_NODE_ENTRIES >= DEFAULT_ALL_ENTRIES);
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::Binary;

    fn node_entry(sequence: u64) -> NodeEntry {
        NodeEntry {
            data: Binary::new(b"payload".to_vec()),
            updated_at_height: 100,
            sequence,
            signature: Binary::new(vec![7u8; 64]),
        }
    }

    fn curated_entry(data: &[u8]) -> CuratedEntry {
        CuratedEntry {
            data: Binary::new(data.to_vec()),
        }
    }

    #[test]
    fn node_entry_round_trip() {
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;

        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .node_entries
                .may_load(store, 1, "sphinx_key")
                .unwrap(),
            None
        );

        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 1, "sphinx_key", &node_entry(0));
        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .node_entries
                .may_load(store, 1, "sphinx_key")
                .unwrap(),
            Some(node_entry(0))
        );

        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .remove(store, 1, "sphinx_key");
        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .node_entries
                .may_load(store, 1, "sphinx_key")
                .unwrap(),
            None
        );
        // removing an absent key is a no-op
        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .remove(store, 1, "sphinx_key");
    }

    #[test]
    fn node_entry_overwrite() {
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;
        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 1, "l", &node_entry(0));
        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 1, "l", &node_entry(1));
        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .node_entries
                .may_load(store, 1, "l")
                .unwrap()
                .unwrap()
                .sequence,
            1
        );
    }

    #[test]
    fn curated_entry_round_trip() {
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;
        NYM_DIRECTORY_CONTRACT_STORAGE.curated_entries.save(
            store,
            "nym-api/1",
            &curated_entry(b"v"),
        );
        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .curated_entries
                .may_load(store, "nym-api/1")
                .unwrap(),
            Some(curated_entry(b"v"))
        );
        NYM_DIRECTORY_CONTRACT_STORAGE
            .curated_entries
            .remove(store, "nym-api/1");
        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .curated_entries
                .may_load(store, "nym-api/1")
                .unwrap(),
            None
        );
    }

    #[test]
    fn node_range_for_single_node_isolates_it() {
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;
        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 7, "a", &node_entry(0));
        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 7, "b", &node_entry(0));
        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 8, "a", &node_entry(0));

        let seven = NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .node_range(store, 7)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            seven.iter().map(|(l, _)| l.as_str()).collect::<Vec<_>>(),
            vec!["a", "b"]
        );
    }

    #[test]
    fn node_range_orders_and_bounds() {
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;
        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 1, "a", &node_entry(0));
        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 2, "b", &node_entry(0));
        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 2, "a", &node_entry(0));

        // full ascending scan, ordered by (node_id, label)
        let all = NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .range(store, None, None, Order::Ascending)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            all.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(),
            vec![
                (1, "a".to_owned()),
                (2, "a".to_owned()),
                (2, "b".to_owned())
            ]
        );

        // descending order is honoured
        let desc = NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .range(store, None, None, Order::Descending)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            desc.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(),
            vec![
                (2, "b".to_owned()),
                (2, "a".to_owned()),
                (1, "a".to_owned())
            ]
        );

        // an exclusive lower bound on the full composite key skips past it (and into
        // the next node) - a label-only bound could not express this
        let rest = NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .range(
                store,
                Some(Bound::exclusive((1u32, "a".to_owned()))),
                None,
                Order::Ascending,
            )
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            rest.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(),
            vec![(2, "a".to_owned()), (2, "b".to_owned())]
        );

        // an inclusive upper bound is respected
        let head = NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .range(
                store,
                None,
                Some(Bound::inclusive((2u32, "a".to_owned()))),
                Order::Ascending,
            )
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            head.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(),
            vec![(1, "a".to_owned()), (2, "a".to_owned())]
        );
    }

    #[test]
    fn curated_range_orders_and_bounds() {
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;
        NYM_DIRECTORY_CONTRACT_STORAGE
            .curated_entries
            .save(store, "a", &curated_entry(b"1"));
        NYM_DIRECTORY_CONTRACT_STORAGE
            .curated_entries
            .save(store, "b", &curated_entry(b"2"));
        NYM_DIRECTORY_CONTRACT_STORAGE
            .curated_entries
            .save(store, "c", &curated_entry(b"3"));

        let all = NYM_DIRECTORY_CONTRACT_STORAGE
            .curated_entries
            .range(store, None, None, Order::Ascending)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            all.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );

        // exclusive lower bound on the key string
        let rest = NYM_DIRECTORY_CONTRACT_STORAGE
            .curated_entries
            .range(
                store,
                Some(Bound::exclusive("a".to_owned())),
                None,
                Order::Ascending,
            )
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            rest.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
            vec!["b", "c"]
        );
    }

    #[test]
    fn stores_do_not_collide_with_other_namespaces() {
        // the two entry stores share the underlying KV store with the sequence and
        // label maps; writes to one must not disturb the others.
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;
        NYM_DIRECTORY_CONTRACT_STORAGE
            .sequences
            .save(store, 1, &42u64)
            .unwrap();
        NYM_DIRECTORY_CONTRACT_STORAGE
            .allowed_storage_labels
            .save(
                store,
                "sphinx_key".to_owned(),
                &LabelConfig { max_size: 256 },
            )
            .unwrap();

        NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .save(store, 1, "sphinx_key", &node_entry(5));
        NYM_DIRECTORY_CONTRACT_STORAGE
            .curated_entries
            .save(store, "nym-api", &curated_entry(b"k"));

        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .current_sequence(store, 1)
                .unwrap(),
            42
        );
        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .allowed_storage_labels
                .load(store, "sphinx_key".to_owned())
                .unwrap()
                .max_size,
            256
        );
        assert!(NYM_DIRECTORY_CONTRACT_STORAGE
            .node_entries
            .may_load(store, 1, "sphinx_key")
            .unwrap()
            .is_some());
        assert!(NYM_DIRECTORY_CONTRACT_STORAGE
            .curated_entries
            .may_load(store, "nym-api")
            .unwrap()
            .is_some());
        // exactly one entry in each store, nothing leaked between them
        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .node_entries
                .range(store, None, None, Order::Ascending)
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            NYM_DIRECTORY_CONTRACT_STORAGE
                .curated_entries
                .range(store, None, None, Order::Ascending)
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
                .len(),
            1
        );
    }

    // ---- digest-maintaining mutations ----

    fn node_leaf(node_id: NodeId, label: &str, entry: &NodeEntry) -> Vec<u8> {
        DirectoryEntryRecord::new_node(node_id, label.to_owned(), entry.clone()).digest_leaf()
    }

    fn curated_leaf(key: &str, entry: &CuratedEntry) -> Vec<u8> {
        DirectoryEntryRecord::new_curated(key.to_owned(), entry.clone()).digest_leaf()
    }

    #[test]
    fn set_and_remove_node_entry_maintains_digest() {
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;
        let s = &NYM_DIRECTORY_CONTRACT_STORAGE;

        // a fresh contract has the empty accumulator
        assert_eq!(s.load_digest(store).unwrap(), LtHash16::new());

        let entry = node_entry(0);
        s.set_node_entry(store, 1, "a", entry.clone()).unwrap();

        let mut expected = LtHash16::new();
        expected.add(&node_leaf(1, "a", &entry));
        assert_eq!(s.load_digest(store).unwrap(), expected);
        assert_eq!(s.node_entries.may_load(store, 1, "a").unwrap(), Some(entry));

        // removing it returns the digest to empty
        s.remove_node_entry(store, 1, "a").unwrap();
        assert_eq!(s.load_digest(store).unwrap(), LtHash16::new());
        assert_eq!(s.node_entries.may_load(store, 1, "a").unwrap(), None);
    }

    #[test]
    fn overwrite_node_entry_retires_the_old_leaf() {
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;
        let s = &NYM_DIRECTORY_CONTRACT_STORAGE;

        s.set_node_entry(store, 1, "a", node_entry(0)).unwrap();
        let new = node_entry(1);
        s.set_node_entry(store, 1, "a", new.clone()).unwrap();

        // the digest commits only the current entry, not the replaced one
        let mut expected = LtHash16::new();
        expected.add(&node_leaf(1, "a", &new));
        assert_eq!(s.load_digest(store).unwrap(), expected);
    }

    #[test]
    fn set_and_remove_curated_entry_maintains_digest() {
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;
        let s = &NYM_DIRECTORY_CONTRACT_STORAGE;

        let entry = curated_entry(b"v");
        s.set_curated_entry(store, "nym-api/1", entry.clone())
            .unwrap();

        let mut expected = LtHash16::new();
        expected.add(&curated_leaf("nym-api/1", &entry));
        assert_eq!(s.load_digest(store).unwrap(), expected);

        s.remove_curated_entry(store, "nym-api/1").unwrap();
        assert_eq!(s.load_digest(store).unwrap(), LtHash16::new());
        assert_eq!(
            s.curated_entries.may_load(store, "nym-api/1").unwrap(),
            None
        );
    }

    #[test]
    fn digest_commits_the_whole_entry_set() {
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;
        let s = &NYM_DIRECTORY_CONTRACT_STORAGE;

        let n = node_entry(0);
        let c = curated_entry(b"v");
        s.set_node_entry(store, 1, "a", n.clone()).unwrap();
        s.set_curated_entry(store, "k", c.clone()).unwrap();

        // multiset over both entries (order-independent)
        let mut both = LtHash16::new();
        both.add(&node_leaf(1, "a", &n));
        both.add(&curated_leaf("k", &c));
        assert_eq!(s.load_digest(store).unwrap(), both);

        // removing the node leaves exactly the curated leaf
        s.remove_node_entry(store, 1, "a").unwrap();
        let mut only_curated = LtHash16::new();
        only_curated.add(&curated_leaf("k", &c));
        assert_eq!(s.load_digest(store).unwrap(), only_curated);
    }

    #[test]
    fn removing_an_absent_entry_leaves_the_digest_untouched() {
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;
        let s = &NYM_DIRECTORY_CONTRACT_STORAGE;

        s.remove_node_entry(store, 9, "missing").unwrap();
        s.remove_curated_entry(store, "missing").unwrap();
        assert_eq!(s.load_digest(store).unwrap(), LtHash16::new());
    }

    #[test]
    fn remove_all_node_entries_clears_one_node_in_a_single_update() {
        let mut deps = mock_dependencies();
        let store = &mut deps.storage;
        let s = &NYM_DIRECTORY_CONTRACT_STORAGE;

        let seven_a = node_entry(0);
        let seven_b = node_entry(1);
        let eight_a = node_entry(0);
        let cur = curated_entry(b"v");
        s.set_node_entry(store, 7, "a", seven_a).unwrap();
        s.set_node_entry(store, 7, "b", seven_b).unwrap();
        s.set_node_entry(store, 8, "a", eight_a.clone()).unwrap();
        s.set_curated_entry(store, "k", cur.clone()).unwrap();

        s.remove_all_node_entries(store, 7).unwrap();

        // node 7 is gone; node 8 and the curated entry remain
        assert!(s
            .node_entries
            .node_range(store, 7)
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .is_empty());
        assert!(s.node_entries.may_load(store, 8, "a").unwrap().is_some());
        assert!(s.curated_entries.may_load(store, "k").unwrap().is_some());

        // the digest commits exactly the surviving entries
        let mut expected = LtHash16::new();
        expected.add(&node_leaf(8, "a", &eight_a));
        expected.add(&curated_leaf("k", &cur));
        assert_eq!(s.load_digest(store).unwrap(), expected);

        // idempotent: clearing a node with no entries changes nothing
        s.remove_all_node_entries(store, 7).unwrap();
        assert_eq!(s.load_digest(store).unwrap(), expected);
    }
}
